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

class ExopticonWorker(object):
    def __init__(self, handle_frame=None, setup=None, cleanup=None, state={}):
        self.handle_frame_callback = self.builtin_handle_frame_callback
        self.setup_callback = self.builtin_setup_callback
        self.cleanup_callback = self.builtin_cleanup_callback
        self.state = state
        self.frame_times = []

        if handle_frame:
            self.handle_frame_callback = partial(handle_frame, self)
        if setup:
            self.setup_callback = partial(setup, self)
        if cleanup:
            self.cleanup_callback = partial(cleanup, self)

    def builtin_setup_callback(self):
        self.log_info('running builtin setup');

    def builtin_handle_frame_callback(self, frame):
        self.log_info('frame size' + str(frame.shape))

    def builtin_cleanup_callback(self):
        self.log_info('analysis worker cleaning up.')

    def setup(self):
        self.setup_callback()

    def cleanup(self):
        self.cleanup_callback()

    def handle_frame(self, frame):
        start_time = time.monotonic()
        self.handle_frame_callback(frame)
        duration = int((time.monotonic() - start_time) * 1000 * 1000)
        self.frame_times.append(duration)
        if len(self.frame_times) >= 10:
            self.write_timing('frame', self.frame_times)
            self.frame_times = []
        #self.log_info('Ran for :' + str(duration * 1000) + ' ms')

    def log_info(self, message):
        log_dict = [0, [message]]
        serialized = msgpack.packb(log_dict, use_bin_type=True)
        self.write_framed_message(serialized)

    def request_frame(self):
        request = [1, [1]]
        serialized = msgpack.packb(request, use_bin_type=True)
        self.write_framed_message(serialized)

    def read_frame(self):
        len_buf = sys.stdin.buffer.read(4)
        msg_len = struct.unpack('>L', len_buf)[0]
        msg_buf = sys.stdin.buffer.read(msg_len)
        msg = msgpack.unpackb(msg_buf, raw=False)
        self.current_frame = msg[1][0]
        msg_buf = numpy.frombuffer(msg[1][0]["jpeg"], dtype=numpy.uint8)
        return cv2.imdecode(msg_buf, cv2.IMREAD_UNCHANGED)

    def write_frame(self, tag, image):
        if not self.current_frame:
            return
        frame = copy.copy(self.current_frame)
        jpeg = cv2.imencode('.jpg', image)[1].tobytes()
        frame_dict = [3, [tag, jpeg]]
        serialized = msgpack.packb(frame_dict, use_bin_type=True)
        self.write_framed_message(serialized)

    def write_timing(self, tag, times):
        timing_dict = [4, [tag, times]]
        serialized = msgpack.packb(timing_dict, use_bin_type=True)
        self.write_framed_message(serialized)

    def write_framed_message(self, serialized):
        packed_len = struct.pack('>L', len(serialized))
        sys.stdout.buffer.write(packed_len)
        sys.stdout.buffer.write(serialized)
        sys.stdout.buffer.flush()

    def run(self):
        self.setup()
        try:
            while True:
                self.request_frame()
                frame = self.read_frame()
                self.handle_frame(frame)
        except EOFerror:
            self.cleanup()
            sys.exit(0)
# End ExopticonWorker
