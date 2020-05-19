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
import json
import base64

class AnalysisMask(object):
    def __init__(self, ul_x, ul_y, lr_x, lr_y):
        self.ul_x = ul_x
        self.ul_y = ul_y
        self.lr_x = lr_x
        self.lr_y = lr_y

class AnalysisFrame(object):
    def __init__(self, m):
        f = m["frame"]

        self.camera_id = f["camera_id"]
        self.video_unit_id = f["video_unit_id"]
        self.unscaled_height = f["unscaled_height"]
        self.unscaled_width = f["unscaled_width"]
        self.offset = f["offset"]
        self.__load_image(f["jpeg"])
        self.__load_masks(m["masks"])

    def __load_image(self, image_str):
        msg_buf = base64.standard_b64decode(image_str)
        msg_buf = numpy.frombuffer(msg_buf, dtype=numpy.uint8)

        self.image = cv2.imdecode(msg_buf, cv2.IMREAD_COLOR)
        self.height, self.width, self.channels = self.image.shape

    def __load_masks(self, raw_masks):
        y_scale = self.height / self.unscaled_height
        x_scale = self.width / self.unscaled_width
        self.masks = []

        for m in raw_masks:
            self.masks.append(AnalysisMask(int(m["ul_x"] * x_scale),
                                           int(m["ul_y"] * y_scale),
                                           int(m["lr_x"] * x_scale),
                                           int(m["lr_y"] * y_scale)))

class WorkerHandler(logging.Handler):
    def __init__(self, worker):
        super().__init__()
        self.worker = worker
    def emit(self, record):
        log_entry = self.format(record)
        self.worker.raw_log(20, log_entry)

class ExopticonWorker(object):
    # Implement private methods
    def __init__(self, worker_name):
        self.__worker_name = worker_name
        self.__current_frame = None
        self.__frame_times = []
        self.__stdout = sys.stdout

    def __setup(self):
        # configure logging
        self.logger = logging.getLogger(self.__worker_name)
        self.logger.setLevel(logging.DEBUG)
        self.__log_handler = WorkerHandler(self)
        self.__log_handler.setLevel(logging.DEBUG)
        self.logger.addHandler(self.__log_handler)

        # Reopen stdout(fd 1)
        newfd = os.dup(1)
        # Open the real stdout(now fd 3)
        self.__stdout = os.fdopen(newfd, mode='wb')
        # Make fd 1 output to stderr, now when anyone writes to fd 1
        # it will go to stderr instead. But we have the read stderr in
        # newfd.
        os.dup2(2, 1)

        # call subclass setup
        self.logger.info("Starting worker...")
        self.setup()

    def __cleanup(self):
        self.cleanup()

    def raw_log(self, level_number, message):
        log_dict = {'Log': {'level': 'Info', 'message': message}}
        serialized = json.dumps(log_dict)
        self.__write_framed_message(serialized)

    def __request_frame(self):
        request = {'FrameRequest': 1}
        serialized = json.dumps(request)
        self.__write_framed_message(serialized)

    def __read_frame(self):
        len_buf = sys.stdin.buffer.read(4)
        msg_len = struct.unpack('>L', len_buf)[0]
        msg_buf = sys.stdin.buffer.read(msg_len)
        msg = json.loads(msg_buf)
        self.__current_frame = AnalysisFrame(msg["Frame"])

        return self.__current_frame

    def __apply_masks(self, frame):
        c = (255,255,255)
        for m in self.__current_frame.masks:
            cv2.rectangle(frame.image, (m.ul_x, m.ul_y), (m.lr_x, m.lr_y), c, -1)

    def write_frame(self, tag, image):
        if not self.__current_frame:
            return

        jpeg = cv2.imencode('.jpg', image)[1].tobytes()
        frame_dict = {'FrameReport': { 'tag': tag, 'jpeg': jpeg }}
        serialized = json.dumps(frame_dict)
        self.__write_framed_message(serialized)

    def __write_timing(self, tag, times):
        timing_dict = { 'TimingReport': { 'tag': tag, 'times': times }}
        serialized = json.dumps(timing_dict)
        self.__write_framed_message(serialized)

    def write_observations(self, observations):
        self.logger.info('observations: ' + str(observations))
        observations_dict = {'Observation': observations}
        serialized = json.dumps(observations_dict)
        self.__write_framed_message(serialized)

    def __write_framed_message(self, serialized):
        serialized = serialized.encode('utf-8')
        packed_len = struct.pack('>L', len(serialized))
        self.__stdout.write(packed_len)
        self.__stdout.write(serialized)
        self.__stdout.flush()

    def __handle_frame(self, frame):
        start_time = time.monotonic()
        self.handle_frame(frame)
        duration = int((time.monotonic() - start_time) * 1000 * 1000)
        self.__frame_times.append(duration)
        if len(self.__frame_times) >= 100:
            self.__write_timing('frame', self.__frame_times)
            self.__frame_times = []
        #self.log_info('Ran for :' + str(duration * 1000) + ' ms')

    @staticmethod
    def get_data_dir():
        return os.path.join(sys.path[0], "data")

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
                self.__apply_masks(frame)
                self.__handle_frame(frame)
        except EOFError:
            sys.exit(0)
        finally:
            self.__cleanup()
