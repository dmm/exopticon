#!/usr/bin/python3

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
