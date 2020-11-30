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

    def handle_frame(self, frame):
        self.interpreter.invoke()
        image = cv2.cvtColor(frame.image, cv2.COLOR_BGR2RGB)
        image = Image.fromarray(image)
        _, scale = common.set_resized_input(
            self.interpreter, image.size, lambda size: image.resize(size, Image.ANTIALIAS))
        objs = detect.get_objects(self.interpreter, 0.5,  scale)

        observations = []
        for o in objs:
            observations.append({
                "videoUnitId": frame.video_unit_id,
                "frameOffset": frame.offset,
                "tag": "object",
                "details": self.labels.get(o.id, o.id),
                "score": int(o.score * 100),
                "ulX": max(int(o.bbox.xmin), 0),
                "ulY": max(int(o.bbox.ymin), 0),
                "lrX": int(o.bbox.xmax),
                "lrY": int(o.bbox.ymax)
            })

        if len(observations) > 0:
            self.write_observations(frame, observations)

if __name__ == "__main__":
    worker = CoralWorker()
    worker.run()
