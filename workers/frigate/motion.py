#!/usr/bin/python3

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


import cv2
import numpy as np
import itertools

from exopticon import ExopticonWorker

from frigate.data.motion import MotionConfig, MotionDetector

class MotionWorker(ExopticonWorker):
    def __init__(self):
        super().__init__("MotionWorker")
#        super().set_frame_size(640, 360)
        self.setup = False

    def config(self, frame):
        frame_shape = frame.image.shape[:2]
        config = MotionConfig(frame_shape)
        self.motion = MotionDetector(frame_shape, config)
        self.setup = True

    def handle_frame(self, frame):

        if not self.setup:
            self.config(frame)

        observations = []
        motion_boxes = self.motion.detect(frame.image)

        # compute regions
        regions = [self.calculate_region(frame.image.shape[:2], a[0], a[1], a[2], a[3], 1.0)
                   for a in motion_boxes]

        # combine overlapping regions
        combined_regions = self.reduce_boxes(regions)
        for box in combined_regions:
            observations.append({
                "videoUnitId": frame.video_unit_id,
                "frameOffset": frame.offset,
                "tag": "motion",
                "details": "motion",
                "score": 100,
                "ulX": int(box[0]),
                "ulY": int(box[1]),
                "lrX": int(box[2]),
                "lrY": int(box[3])
            })
        if len(motion_boxes) > 0:
            self.write_observations(frame, observations)

    def calculate_region(self, frame_shape, xmin, ymin, xmax, ymax, multiplier=2):
        # size is the longest edge and divisible by 4
        size = int(max(xmax-xmin, ymax-ymin)//4*4*multiplier)
        # dont go any smaller than 300
        if size < 300:
            size = 300

        # x_offset is midpoint of bounding box minus half the size
        x_offset = int((xmax-xmin)/2.0+xmin-size/2.0)
        # if outside the image
        if x_offset < 0:
            x_offset = 0
        elif x_offset > (frame_shape[1]-size):
            x_offset = max(0, (frame_shape[1]-size))

        # y_offset is midpoint of bounding box minus half the size
        y_offset = int((ymax-ymin)/2.0+ymin-size/2.0)
        # # if outside the image
        if y_offset < 0:
            y_offset = 0
        elif y_offset > (frame_shape[0]-size):
            y_offset = max(0, (frame_shape[0]-size))

        return (x_offset, y_offset, x_offset+size, y_offset+size)

    def reduce_boxes(self, boxes):
        if len(boxes) == 0:
            return []
        reduced_boxes = cv2.groupRectangles([list(b) for b in itertools.chain(boxes, boxes)], 1, 0.2)[0]
        return [tuple(b) for b in reduced_boxes]

if __name__ == "__main__":
    worker = MotionWorker()
    worker.run()
