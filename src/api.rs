use rand::{self, rngs::ThreadRng, Rng};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use actix::*;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use log::{error, info};

use crate::audiosys::AudioFeatures;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

async fn websocket(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<ApiServer>>,
) -> Result<HttpResponse, Error> {
    ws::start(
        WsSession {
            id: 0,
            hb: Instant::now(),
            addr: srv.get_ref().clone(),
        },
        &req,
        stream,
    )
}

pub struct WsSession {
    id: usize,
    hb: Instant,
    addr: Addr<ApiServer>,
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);

        let addr = ctx.address();
        self.addr
            .send(Connect {
                addr: addr.recipient(),
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(res) => act.id = res,
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        self.addr.do_send(Disconnect { id: self.id });
        Running::Stop
    }
}

impl Handler<Message> for WsSession {
    type Result = ();

    fn handle(&mut self, msg: Message, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let msg = match msg {
            Err(_) => {
                ctx.stop();
                return;
            }
            Ok(msg) => msg,
        };

        match msg {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.hb = Instant::now();
            }
            ws::Message::Text(text) => {
                let m = text.trim();
                println!("websocket text: {}", m);
                if m.starts_with('/') {
                    let v: Vec<&str> = m.splitn(3, '/').collect();
                    match v[1] {
                        "subscribe" => match v[2] {
                            "audio" => {
                                info!("enabled audio subscribe for session {}", self.id);
                                self.addr
                                    .send(Subscribe {
                                        id: self.id,
                                        sub: Subscription::AudioFeatures(ctx.address().recipient()),
                                    })
                                    .into_actor(self)
                                    .then(|res, _, _| {
                                        if let Err(e) = res {
                                            error!("failed to send subscribe req: {}", e);
                                        }
                                        fut::ready(())
                                    })
                                    .wait(ctx);
                            }
                            _ => (),
                        },
                        _ => (),
                    }
                }
            }
            ws::Message::Binary(_) => error!("Unexpected binary"),
            ws::Message::Close(reason) => {
                ctx.close(reason);
                ctx.stop();
            }
            ws::Message::Continuation(_) => {
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}

impl Handler<AudioMessage> for WsSession {
    type Result = ();

    fn handle(&mut self, msg: AudioMessage, ctx: &mut Self::Context) {
        let js = serde_json::to_string(&msg.0).unwrap();
        ctx.text(js);
        // let ms = rmp_serde::to_vec(&msg.0).unwrap();
        // ctx.binary(ms);
    }
}

impl WsSession {
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                act.addr.do_send(Disconnect { id: act.id });
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Message(pub String);

#[derive(Message)]
#[rtype(usize)]
struct Connect {
    addr: Recipient<Message>,
}

#[derive(Message)]
#[rtype(result = "()")]
struct Disconnect {
    id: usize,
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub(crate) struct AudioMessage(pub AudioFeatures);

#[derive(Message)]
#[rtype(result = "()")]
struct Subscribe {
    id: usize,
    sub: Subscription,
}

enum Subscription {
    AudioFeatures(Recipient<AudioMessage>),
}

#[derive(Message)]
#[rtype(result = "()")]
struct Unsubscribe {
    id: usize,
    sub: Subscription,
}

pub struct ApiServer {
    sessions: HashMap<usize, Recipient<Message>>,
    rng: ThreadRng,
    app: Addr<MainApp>,
    audio_subs: HashMap<usize, Recipient<AudioMessage>>,
}

use super::App as MainApp;

impl ApiServer {
    pub fn new(app: Addr<MainApp>) -> Self {
        Self {
            sessions: HashMap::new(),
            rng: rand::thread_rng(),
            app,
            audio_subs: HashMap::new(),
        }
    }

    fn disable_audio_subscriptions(&self, ctx: &mut Context<Self>) {
        self.app
            .send(AppConfigMessage(AppSystemConfig {
                subscription_enabled: Some(false),
                apiserver: None,
            }))
            .into_actor(self)
            .then(|res, _, _| {
                if let Err(e) = res {
                    println!("send sub disable error: {}", e);
                }
                fut::ready(())
            })
            .wait(ctx);
    }
}

impl Actor for ApiServer {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.app
            .send(AppConfigMessage(AppSystemConfig {
                subscription_enabled: None,
                apiserver: Some(ctx.address()),
            }))
            .into_actor(self)
            .then(|res, _, _| {
                if let Err(e) = res {
                    println!("send appconfig error: {}", e);
                }
                fut::ready(())
            })
            .wait(ctx);
    }
}

impl Handler<Connect> for ApiServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Self::Context) -> Self::Result {
        let id = self.rng.gen::<usize>();
        self.sessions.insert(id, msg.addr);
        id
    }
}

impl Handler<Disconnect> for ApiServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, ctx: &mut Self::Context) {
        let _ = self.sessions.remove(&msg.id);
        let _ = self.audio_subs.remove(&msg.id);
        if self.audio_subs.len() == 0 {
            self.disable_audio_subscriptions(ctx);
        }
    }
}

impl Handler<AudioMessage> for ApiServer {
    type Result = ();

    fn handle(&mut self, msg: AudioMessage, ctx: &mut Self::Context) {
        for addr in self.audio_subs.values() {
            addr.send(msg.clone())
                .into_actor(self)
                .then(|res, _, _| {
                    if let Err(e) = res {
                        error!("failed to relay audio to subscriber: {}", e);
                    }
                    fut::ready(())
                })
                .wait(ctx);
        }
    }
}

use crate::app::{AppConfigMessage, AppSystemConfig};

impl Handler<Subscribe> for ApiServer {
    type Result = ();

    fn handle(&mut self, msg: Subscribe, ctx: &mut Self::Context) {
        match msg.sub {
            Subscription::AudioFeatures(addr) => {
                let _ = self.audio_subs.insert(msg.id, addr);
                self.app
                    .send(AppConfigMessage(AppSystemConfig {
                        subscription_enabled: Some(true),
                        apiserver: None,
                    }))
                    .into_actor(self)
                    .then(|res, _, _| {
                        if let Err(e) = res {
                            println!("send sub enable error: {}", e);
                        }
                        fut::ready(())
                    })
                    .wait(ctx);
            }
        }
    }
}

impl Handler<Unsubscribe> for ApiServer {
    type Result = ();

    fn handle(&mut self, msg: Unsubscribe, ctx: &mut Self::Context) {
        match msg.sub {
            Subscription::AudioFeatures(addr) => {
                let _ = self.audio_subs.remove(&msg.id);
                if self.audio_subs.len() == 0 {
                    self.disable_audio_subscriptions(ctx);
                }
            }
        }
    }
}

pub async fn run(addr: &str, port: &str, app: Addr<MainApp>) -> std::io::Result<()> {
    let server = ApiServer::new(app).start();

    HttpServer::new(move || {
        App::new()
            .data(server.clone())
            .service(web::resource("/api/v1/ws/").to(websocket))
    })
    .bind(format!("{}:{}", addr, port))?
    .run()
    .await
}
