#!/usr/bin/python3

import numpy as np

from exopticon import ExopticonWorker

from frigate.frigate.frigate.motion import MotionDetector

class MotionWorker(ExopticonWorker):
    def __init__(self):
        super().__init__("MotionWorker")

    def setup(self):
        # we don't know the frame size yet so just set a flag
        self.setup = False

    def config(self, frame):
        frame_shape = frame.image.shape[:2]
        mask = np.zeros((frame_shape[0], frame_shape[1], 1), np.uint8)
        mask[:] = 255
        self.motion = MotionDetector(frame_shape, mask, 2)
        self.setup = True

    def handle_frame(self, frame):
        if not self.setup:
            self.config(frame)

        observations = []
        motion_boxes = self.motion.detect(frame.image)
        x_scale = frame.unscaled_width / frame.width
        y_scale = frame.unscaled_height / frame.height

        for box in motion_boxes:

            observations.append({
                "videoUnitId": frame.video_unit_id,
                "frameOffset": frame.offset,
                "tag": "motion",
                "details": "motion",
                "score": 100,
                "ulX": int(box[0] * x_scale),
                "ulY": int(box[1] * y_scale),
                "lrX": int(box[2] * x_scale),
                "lrY": int(box[3] * y_scale)
            })
        if len(motion_boxes) > 0:
            self.logger.info("Found " + str(len(motion_boxes)) + " boxes!")
            self.write_observations(observations)



if __name__ == "__main__":
    worker = MotionWorker()
    worker.run()
