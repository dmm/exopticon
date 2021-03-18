
# Exopticon - A free video surveillance system.
# Copyright (C) 2020 David Matthew Mattli <dmm@mattli.us>
#
# This file is part of Exopticon.
#
# Exopticon is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# Exopticon is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with Exopticon.  If not, see <http://www.gnu.org/licenses/>.

import copy
import cv2
import msgpack
import numpy as np
import struct
import sys
import time
from functools import partial
import os
import argparse
import logging
import json
import base64
import math

class AnalysisMask(object):
    def __init__(self, ul_x, ul_y, lr_x, lr_y):
        self.ul_x = ul_x
        self.ul_y = ul_y
        self.lr_x = lr_x
        self.lr_y = lr_y

class Observation(object):
    def __init__(self, id, tag, details, score, ul_x, ul_y, lr_x, lr_y):
        self.id = id
        self.tag = tag
        self.details = details
        self.score = score
        self.ul_x = ul_x
        self.ul_y = ul_y
        self.lr_x = lr_x
        self.lr_y = lr_y

def scale_observations(x_scale, y_scale, observations):
    scaled_observations = []
    for o in observations:
        scaled_observations.append(Observation(o.id,
                                               o.tag,
                                               o.details,
                                               o.score,
                                               int(o.ul_x * x_scale),
                                               int(o.ul_y * y_scale),
                                               int(o.lr_x * x_scale),
                                               int(o.lr_y * y_scale)))
    return scaled_observations

class AnalysisFrame(object):
    def __init__(self, m, scaled_width, scaled_height):
        self.__scaled_width = scaled_width
        self.__scaled_height = scaled_height
        f = m["frame"]

        self.camera_id = f["camera_id"]
        self.video_unit_id = f["video_unit_id"]
        self.unscaled_height = f["unscaled_height"]
        self.unscaled_width = f["unscaled_width"]
        self.offset = f["offset"]
        self.__load_image(f["jpeg"])
        self.__load_observations(f["observations"])
        self.__load_masks(m["masks"])

    def __load_image(self, image_str):
        msg_buf = base64.standard_b64decode(image_str)
        msg_buf = np.frombuffer(msg_buf, dtype=np.uint8)

        image = cv2.imdecode(msg_buf, cv2.IMREAD_COLOR)

        if (self.__scaled_width == 0 or self.__scaled_height == 0):
            self.image = image
        else:
            self.image = cv2.resize(image, (self.__scaled_width, self.__scaled_height))

        self.image = cv2.cvtColor(self.image, cv2.COLOR_BGR2RGB)
        self.height, self.width, self.channels = self.image.shape

    def __load_observations(self, raw_observations):
        y_scale = self.height / self.unscaled_height
        x_scale = self.width / self.unscaled_width

        self.observations = []

        for o in raw_observations:
            self.observations.append(Observation(o["id"],
                                                 o["tag"],
                                                 o["details"],
                                                 int(o["score"]),
                                                 o["ulX"],
                                                 int(o["ulY"]),
                                                 int(o["lrX"]),
                                                 int(o["lrY"])))
        self.observations = scale_observations(x_scale, y_scale, self.observations)

    def __load_masks(self, raw_masks):
        y_scale = self.height / self.unscaled_height
        x_scale = self.width / self.unscaled_width
        self.masks = []

        for m in raw_masks:
            self.masks.append(AnalysisMask(int(m["ulX"] * x_scale),
                                           int(m["ulY"] * y_scale),
                                           int(m["lrX"] * x_scale),
                                           int(m["lrY"] * y_scale)))

    def get_observation_bounding_box(self):
        ul_x = 999999999999
        ul_y = 999999999999
        lr_x = -1
        lr_y = -1

        for o in self.observations:
            if o.ul_x < ul_x:
                ul_x = o.ul_x
            if o.ul_y < ul_y:
                ul_y = o.ul_y
            if lr_x < o.lr_x:
                lr_x = o.lr_x
            if lr_y < o.lr_y:
                lr_y = o.lr_y

        return [ul_x, ul_y, lr_x, lr_y]

    def calculate_region(image_dims, region, min_size):
        # [height, width] of region
        rdims = [region[3] - region[1] + 1, region[2] - region[0] + 1]
        new_region = [0, 0, 0, 0]
        # if region.width >= min_size, return actual width
        if rdims[1] >= min_size:
            new_region[0] = region[0]
            new_region[2] = region[2]
        # if region.height >= min_size, return actual height
        if rdims[0] >= min_size:
            new_region[1] = region[1]
            new_region[3] = region[3]
        # if image.width is <= min_size, return whole width
        if image_dims[1] <= min_size:
            new_region[0] = 0
            new_region[2] = image_dims[1] - 1
        # if image.height is <= min_size, return whole height
        if image_dims[0] <= min_size:
            new_region[1] = 0
            new_region[3] = image_dims[0] - 1

        # region.width < min_size, expand region width
        if image_dims[1] > min_size and rdims[1] < min_size:
            expand_size = min_size - rdims[1]
            half_expand = expand_size / 2.0
        #   new_ul_x = region.ul_x + round(half_expand)
            new_region[0] = region[0] - round(half_expand)
        #   new_lr_x = region.lr_x + math.ceil(half_expand)
            new_region[2] = region[2] + math.ceil(half_expand)
            if new_region[0] < 0:
                diff = 0 - new_region[0]
                new_region[2] += diff
                new_region[0] = 0
            elif new_region[2] > image_dims[1] - 1:
                diff = new_region[2] - image_dims[1] - 1
                new_region[2] = image_dims[1] - 1
                new_region[0] -= diff

        # region.height < min_size, expand region height
        if image_dims[0] > min_size and rdims[0] < min_size:
            expand_size = min_size - rdims[0]
            half_expand = expand_size / 2.0
            new_region[1] = region[1] - round(half_expand)
            new_region[3] = region[3] + math.ceil(half_expand)
            if new_region[1] < 0:
                diff = 0 - new_region[1]
                new_region[3] += diff - 1
                new_region[1] = 0
            elif new_region[3] > image_dims[0] - 1:
                diff = new_region[3] - image_dims[0] - 1
                new_region[3] = image_dims[0] - 1
                new_region[1] -= diff

        return new_region

    # Returns slice of image data
    def get_region(self, region, min_size=0):

        new_region = AnalysisFrame.calculate_region(self.image.shape, region, min_size)
        image = self.image[new_region[1]:new_region[3], new_region[0]:new_region[2]]
        offset = [new_region[1], new_region[3]]
        return FrameSlice(image, offset)

class FrameSlice(object):
    def __init__(self, image, offset):
        self.image = image
        self.offset = offset

class WorkerHandler(logging.Handler):
    def __init__(self, worker):
        super().__init__()
        self.worker = worker
    def emit(self, record):
        log_entry = self.format(record)
        self.worker.raw_log(record.levelno, log_entry)

class ExopticonWorker(object):
    def __init__(self, worker_name):
        self.__worker_name = worker_name
        self.__frame_width = 0
        self.__frame_height = 0
        self.__current_frame = None
        self.__frame_times = []
        self.__stdout = sys.stdout

        # configure logging
        self.logger = logging.getLogger(self.__worker_name)
        self.logger.setLevel(logging.DEBUG)
        self.__log_handler = WorkerHandler(self)
        self.__log_handler.setLevel(logging.DEBUG)
        self.logger.addHandler(self.__log_handler)

        # Configure exception hook to capture an uncaught exception
        sys.excepthook = lambda x, y, z: self.__log_exception(x, y, z)

        # Reopen stdout(fd 1)
        newfd = os.dup(1)
        # Open the real stdout(now fd 3)
        self.__stdout = os.fdopen(newfd, mode='wb')
        # Make fd 1 output to stderr, now when anyone writes to fd 1
        # it will go to stderr instead. But we have the read stderr in
        # newfd.
        os.dup2(2, 1)

        self.logger.info("Starting worker...")

    def set_frame_size(self, frame_width, frame_height):
        self.__frame_width = frame_width
        self.__frame_height = frame_height

    # Implement private methods
    def __log_exception(self, exception_type, value, tb):
        self.logger.error("Capture Worker exception: %s %s %s", exception_type, value, tb)

    def __cleanup(self):
        self.cleanup()

    def raw_log(self, level_number, message):
        log_dict = {'Log': {'level': level_number, 'message': message}}
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
        self.__current_frame = AnalysisFrame(msg["Frame"], self.__frame_width, self.__frame_height)

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

    def write_observations(self, frame, observations):
        y_scale = frame.unscaled_height / frame.height
        x_scale = frame.unscaled_width / frame.width
        for o in observations:
            o["ulX"] = int(o["ulX"] * x_scale)
            o["ulY"] = int(o["ulY"] * y_scale)
            o["lrX"] = int(o["lrX"] * x_scale)
            o["lrY"] = int(o["lrY"] * y_scale)
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
    def cleanup(self):
        []
    def handle_frame(self, frame):
        []

    def run(self):
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
