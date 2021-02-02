import threading
import random
from itertools import count
import matplotlib.pyplot as plt
from matplotlib.animation import FuncAnimation
import asyncio
import websockets
import json

BINS = 16
LENGTH = 1024


def csv_fields():
    def amp_field(i: int):
        return (f"amp_{i}", lambda f: f["amplitudes"][i])

    fields = [amp_field(i) for i in range(BINS)]

    return (list(map(lambda f: f[0], fields)), list(map(lambda f: f[1], fields)))


index = count()
x_values = [next(index) for _ in range(LENGTH)]
y_values = [[0]*8 for _ in range(LENGTH)]  # [0]*BINS


async def connect(uri):
    (headers, getters) = csv_fields()

    async with websockets.connect(uri) as websocket:

        # print(','.join(headers))

        await websocket.send("/sub/audio")
        while 1:
            data = await websocket.recv()
            audio_data = json.loads(data)["Audio"]
            features, state = audio_data[0], audio_data[1]
            x_values.pop(0)
            y_values.pop(0)
            x_values.append(next(index))
            y_values.append(state["fs"]["gain_controller"]["gain"][:8])

            # print(','.join(str(g(features)) for g in getters))


def loop_in_thread(loop):
    asyncio.set_event_loop(loop)
    loop.run_until_complete(connect('ws://127.0.0.1:8080/api/v1/ws/'))


loop = asyncio.get_event_loop()
t = threading.Thread(target=loop_in_thread, args=(loop,))
t.start()

# asyncio.get_event_loop().run_until_complete(
#     connect('ws://127.0.0.1:8080/api/v1/ws/'))


def animate(i):
    plt.cla()
    plt.plot(x_values, y_values)


ani = FuncAnimation(plt.gcf(), animate, interval=10)


# plt.style.use('seaborn-dark-palette')
# plt.tight_layout()
plt.show()
