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
import imutils
import itertools

from exopticon import ExopticonWorker

# MotionDetector borrowed and lightly modified from Frigate
class MotionConfig():
    def __init__(self, frame_shape):
        default_mask = np.zeros(frame_shape, np.uint8)
        default_mask[:] = 255
        self._mask = default_mask
        self._threshold = 25
        self._contour_area = 100
        self._delta_alpha = 0.2
        self._frame_alpha = 0.2
        self._frame_height = frame_shape[0]

    @property
    def mask(self):
        return self._mask

    @property
    def threshold(self):
        return self._threshold

    @property
    def contour_area(self):
        return self._contour_area

    @property
    def delta_alpha(self):
        return self._delta_alpha

    @property
    def frame_alpha(self):
        return self._frame_alpha

    @property
    def frame_height(self):
        return self._frame_height

    def to_dict(self):
        return {
            'mask': self._raw_mask,
            'threshold': self.threshold,
            'contour_area': self.contour_area,
            'delta_alpha': self.delta_alpha,
            'frame_alpha': self.frame_alpha,
            'frame_height': self.frame_height,
        }

class MotionDetector():
    def __init__(self, frame_shape):
        config = MotionConfig(frame_shape)
        self.config = config
        self.frame_shape = frame_shape
        self.resize_factor = frame_shape[0]/config.frame_height
        self.motion_frame_size = (config.frame_height, config.frame_height*frame_shape[1]//frame_shape[0])
        self.avg_frame = np.zeros(self.motion_frame_size, np.float)
        self.avg_delta = np.zeros(self.motion_frame_size, np.float)
        self.motion_frame_count = 0
        self.frame_counter = 0
        resized_mask = cv2.resize(config.mask, dsize=(self.motion_frame_size[1], self.motion_frame_size[0]), interpolation=cv2.INTER_LINEAR)
        self.mask = np.where(resized_mask==[0])

    def detect(self, frame):
        motion_boxes = []

        gray = frame[0:self.frame_shape[0], 0:self.frame_shape[1]]

        # resize frame
#        resized_frame = cv2.resize(gray, dsize=(self.motion_frame_size[1], self.motion_frame_size[0]), interpolation=cv2.INTER_LINEAR)

        # TODO: can I improve the contrast of the grayscale image here?

        # convert to grayscale
        resized_frame = cv2.cvtColor(gray, cv2.COLOR_BGR2GRAY)

        # mask frame
        resized_frame[self.mask] = [255]

        # it takes ~30 frames to establish a baseline
        # dont bother looking for motion
        if self.frame_counter < 30:
            self.frame_counter += 1
        else:
            # compare to average
            frameDelta = cv2.absdiff(resized_frame, cv2.convertScaleAbs(self.avg_frame))

            # compute the average delta over the past few frames
            # higher values mean the current frame impacts the delta a lot, and a single raindrop may
            # register as motion, too low and a fast moving person wont be detected as motion
            cv2.accumulateWeighted(frameDelta, self.avg_delta, self.config.delta_alpha)

            # compute the threshold image for the current frame
            # TODO: threshold
            current_thresh = cv2.threshold(frameDelta, self.config.threshold, 255, cv2.THRESH_BINARY)[1]

            # black out everything in the avg_delta where there isnt motion in the current frame
            avg_delta_image = cv2.convertScaleAbs(self.avg_delta)
            avg_delta_image = cv2.bitwise_and(avg_delta_image, current_thresh)

            # then look for deltas above the threshold, but only in areas where there is a delta
            # in the current frame. this prevents deltas from previous frames from being included
            thresh = cv2.threshold(avg_delta_image, self.config.threshold, 255, cv2.THRESH_BINARY)[1]

            # dilate the thresholded image to fill in holes, then find contours
            # on thresholded image
            thresh = cv2.dilate(thresh, None, iterations=2)
            cnts = cv2.findContours(thresh, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
            cnts = imutils.grab_contours(cnts)

            # loop over the contours
            for c in cnts:
                # if the contour is big enough, count it as motion
                contour_area = cv2.contourArea(c)
                if contour_area > self.config.contour_area:
                    x, y, w, h = cv2.boundingRect(c)
                    motion_boxes.append((int(x*self.resize_factor), int(y*self.resize_factor), int((x+w)*self.resize_factor), int((y+h)*self.resize_factor)))

        if len(motion_boxes) > 0:
            self.motion_frame_count += 1
            if self.motion_frame_count >= 10:
                # only average in the current frame if the difference persists for a bit
                cv2.accumulateWeighted(resized_frame, self.avg_frame, self.config.frame_alpha)
        else:
            # when no motion, just keep averaging the frames together
            cv2.accumulateWeighted(resized_frame, self.avg_frame, self.config.frame_alpha)
            self.motion_frame_count = 0

        return motion_boxes

class MotionWorker(ExopticonWorker):
    def __init__(self):
        super().__init__("MotionWorker")
#        super().set_frame_size(640, 360)
        self.setup = False

    def config(self, frame):
        frame_shape = frame.image.shape[:2]
        self.motion = MotionDetector(frame_shape)
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
