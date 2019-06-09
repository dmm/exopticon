#!/usr/bin/python3

import copy
import cv2
import msgpack
import numpy
import struct
import sys
import time
from functools import partial
import imutils

class ExopticonWorker(object):
    def __init__(self, handle_frame=None, setup=None):
        self.handle_frame_callback = self.builtin_handle_frame_callback
        self.setup_callback = self.builtin_setup_callback

        if handle_frame:
            self.handle_frame_callback = partial(handle_frame, self)
        if setup:
            self.setup_callback = partial(setup, self)

    def builtin_setup_callback(self):
        self.state = {}

    def builtin_handle_frame_callback(self, frame):
        log_info('frame size' + str(frame.shape))

    def setup(self):
        self.state = {}
        self.setup_callback()

    def cleanup(self):
        self.log_info("cleaning up!")

    def handle_frame(self, frame):
        start_time = time.monotonic()
        self.handle_frame_callback(frame)
        duration = time.monotonic() - start_time
        self.log_info('Ran for :' + str(duration * 1000) + ' ms')

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

def my_setup(self):
    fgbg = cv2.createBackgroundSubtractorKNN()
    self.state['fgbg'] = fgbg

def my_handle_frame(self, frame):
    #frame = imutils.resize(frame, width=500)
    #gray = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)
    gray = cv2.GaussianBlur(frame, (21, 21), 0)

    fgmask = self.state['fgbg'].apply(gray)
    #self.write_frame("foreground", fgmask)

    # dilate the thresholded image to fill in holes then find contours
    thresh = cv2.dilate(fgmask, None, iterations=2)
    cnts = cv2.findContours(thresh.copy(), cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
    cnts = imutils.grab_contours(cnts)

    # loop over contours
    for c in cnts:
        # ignore small contours
        if cv2.contourArea(c) < 50:
            continue
        # compute the bounding box and draw it on frame
        (x, y, w, h) = cv2.boundingRect(c)
        cv2.rectangle(frame, (x, y), (x+w, y+h), (0, 255, 0), 2)

    self.write_frame("contours", frame)

def main():
    worker = ExopticonWorker(setup=my_setup, handle_frame=my_handle_frame)
    worker.run()

if __name__ == "__main__":
    main()
