import copy
import cv2
import msgpack
import numpy
import struct
import sys
import time
from functools import partial
import os
import argparse
import logging
import cv2

class WorkerHandler(logging.Handler):
    def __init__(self, worker):
        super().__init__()
        self.worker = worker
    def emit(self, record):
        log_entry = self.format(record)
        self.worker.raw_log(record.levelno, log_entry)

class ExopticonWorker(object):
    # Implement private methods
    def __init__(self, worker_name):
        self.__worker_name = worker_name
        self.__current_frame = None
        self.__frame_times = []

    def __setup(self):
        # configure logging
        self.logger = logging.getLogger(self.__worker_name)
        self.logger.setLevel(logging.DEBUG)
        self.__log_handler = WorkerHandler(self)
        self.__log_handler.setLevel(logging.DEBUG)
        self.logger.addHandler(self.__log_handler)

        # call subclass setup
        self.logger.info("Starting worker...")
        self.setup()

    def __cleanup(self):
        self.cleanup()

    def raw_log(self, level_number, message):
        log_dict = [0, [level_number, message]]
        serialized = msgpack.packb(log_dict, use_bin_type=True)
        self.__write_framed_message(serialized)

    def __request_frame(self):
        request = [1, [1]]
        serialized = msgpack.packb(request, use_bin_type=True)
        self.__write_framed_message(serialized)

    def __read_frame(self):
        len_buf = sys.stdin.buffer.read(4)
        msg_len = struct.unpack('>L', len_buf)[0]
        msg_buf = sys.stdin.buffer.read(msg_len)
        msg = msgpack.unpackb(msg_buf, raw=False)
        self.__current_frame = msg[1][0]
        msg_buf = numpy.frombuffer(msg[1][0]["jpeg"], dtype=numpy.uint8)

        img = cv2.imdecode(msg_buf, cv2.IMREAD_UNCHANGED)
        height, width, channels = img.shape

        return dict(camera_id=msg[1][0]["camera_id"],
                    image=img,
                    unscaled_height=msg[1][0]["unscaled_height"],
                    unscaled_width=msg[1][0]["unscaled_width"],
                    video_unit_id=msg[1][0]["video_unit_id"],
                    offset=msg[1][0]["offset"])
#        return cv2.imdecode(msg_buf, cv2.IMREAD_UNCHANGED)

    def write_frame(self, tag, image):
        if not self.__current_frame:
            return
        frame = copy.copy(self.__current_frame)
        jpeg = cv2.imencode('.jpg', image)[1].tobytes()
        frame_dict = [3, [tag, jpeg]]
        serialized = msgpack.packb(frame_dict, use_bin_type=True)
        self.__write_framed_message(serialized)

    def __write_timing(self, tag, times):
        timing_dict = [4, [tag, times]]
        serialized = msgpack.packb(timing_dict, use_bin_type=True)
        self.__write_framed_message(serialized)

    def write_observations(self, observations):
        self.logger.info('observations: ' + str(observations))
        observation_dict = [2, [observations]]
        serialized = msgpack.packb(observation_dict, use_bin_type=True)
        self.__write_framed_message(serialized)

    def __write_framed_message(self, serialized):
        packed_len = struct.pack('>L', len(serialized))
        sys.stdout.buffer.write(packed_len)
        sys.stdout.buffer.write(serialized)
        sys.stdout.buffer.flush()

    def __handle_frame(self, frame):
        start_time = time.monotonic()
        self.handle_frame(frame)
        duration = int((time.monotonic() - start_time) * 1000 * 1000)
        self.__frame_times.append(duration)
        if len(self.__frame_times) >= 100:
            self.__write_timing('frame', self.__frame_times)
            self.__frame_times = []
        #self.log_info('Ran for :' + str(duration * 1000) + ' ms')


    # Implement extendable methods
    def setup(self):
        []
    def cleanup(self):
        []
    def handle_frame(self, frame):
        []

    def run(self):
        self.__setup()
        try:
            while True:
                self.__request_frame()
                frame = self.__read_frame()
                self.__handle_frame(frame)
        except EOFError:
            sys.exit(0)
        finally:
            self.__cleanup()
