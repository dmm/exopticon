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


import numpy as np

from exopticon import ExopticonWorker

from frigate.data.motion import MotionDetector

class MotionWorker(ExopticonWorker):
    def __init__(self):
        super().__init__("MotionWorker")
        super().set_frame_size(320, 180)
        self.setup = False

    def config(self, frame):
        frame_shape = frame.image.shape[:2]
        mask = np.zeros((frame_shape[0], frame_shape[1], 1), np.uint8)
        mask[:] = 255
        self.motion = MotionDetector(frame_shape, mask, 1)
        self.setup = True

    def handle_frame(self, frame):
        if not self.setup:
            self.config(frame)

        observations = []
        motion_boxes = self.motion.detect(frame.image)

        for box in motion_boxes:

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
            self.logger.info("Found " + str(len(motion_boxes)) + " boxes!")
            self.write_observations(frame, observations)



if __name__ == "__main__":
    worker = MotionWorker()
    worker.run()
