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


import os
import cv2
from PIL import Image
from PIL import ImageDraw
from collections import defaultdict

from pycoral.adapters import common
from pycoral.adapters import detect
from pycoral.utils.dataset import read_label_file
from pycoral.utils.edgetpu import make_interpreter

from exopticon import ExopticonWorker

class CoralWorker(ExopticonWorker):
    def __init__(self):
        super().__init__("CoralWorker")

        self.interpreter = make_interpreter(os.path.join(ExopticonWorker.get_data_dir(), "ssd_mobilenet_v2_coco_quant_postprocess_edgetpu.tflite"))
        self.interpreter.allocate_tensors()
        input_size = common.input_size(self.interpreter)
        self.labels = read_label_file(os.path.join(ExopticonWorker.get_data_dir(), "coco_labels.txt"))

    def detect(self, image, offset):
        image = Image.fromarray(image)
        _, scale = common.set_resized_input(
            self.interpreter, image.size, lambda size: image.resize(size, Image.ANTIALIAS))
        self.interpreter.invoke()
        objs = detect.get_objects(self.interpreter, 0.5, scale)

        observations = []
        for o in objs:
            observations.append((
                self.labels.get(o.id, o.id),
                o.score,
                (max(int(o.bbox.xmin * scale[1] + offset[0]), 0),
                 max(int(o.bbox.ymin * scale[0] + offset[1]), 0),
                 int(o.bbox.xmax * scale[1]  + offset[0]),
                 int(o.bbox.ymax * scale[0]  + offset[1]))

            ))
        return observations

    def handle_frame(self, frame):
        image = frame.image
        offset = [0, 0]
        important_labels = ["person", "car", "truck", "dog"]

        detections = []

        for obs in frame.observations:
            slice = frame.get_region(obs.box())
            offset = slice.offset
            image = slice.image
            detections.extend(self.detect(image, offset))


        observations = []
        for o in detections:
            if o[0] in important_labels:
                observations.append({
                    "videoUnitId": frame.video_unit_id,
                    "frameOffset": frame.offset,
                    "tag": "object",
                    "details": o[0],
                    "score": int(o[1] * 100),
                    "ulX": o[2][0],
                    "ulY": o[2][1],
                    "lrX": o[2][2],
                    "lrY": o[2][3]
                })

        if len(observations) > 0:
            self.write_observations(frame, observations)

if __name__ == "__main__":
    worker = CoralWorker()
    worker.run()
