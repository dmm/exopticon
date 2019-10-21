#!/usr/bin/python3

from __future__ import division

from yolov3.models import *
import yolov3.utils.utils as utils
import yolov3.utils.datasets

import os
import sys
import time
import datetime
import argparse
import random

from PIL import Image

import torch
from torch.utils.data import DataLoader
from torchvision import datasets, transforms
from torch.autograd import Variable

import matplotlib.pyplot as plt
import matplotlib.patches as patches
from matplotlib.ticker import NullLocator

import cv2

from exopticon_worker import ExopticonWorker

class Yolo3Worker(ExopticonWorker):
    def __init__(self):
        super().__init__("Yolo3Worker")

    def setup(self):
        self.logger.info('starting yolov3 worker!')
        os.chdir('workers/yolov3')
        self.logger.info('cwd: ' + os.getcwd())

        device = torch.device('cuda' if torch.cuda.is_available() else 'cpu')
        model = Darknet('config/yolov3.cfg', 416)
        # load weights
        model.load_darknet_weights('weights/yolov3.weights')

        # set model in evaluation mode
        model.eval()
        model.cuda()

        classes = utils.load_classes('data/coco.names')

        tensor = torch.cuda.FloatTensor if torch.cuda.is_available() else torch.FloatTensor

        self.model = model
        self.classes = classes
        self.tensor = tensor
        self.img_size = 416
        self.conf_thres = 0.8
        self.nms_thres = 0.4

    def detect_image(self, img):
        # scale and pad image
        img_size = self.img_size
        ratio = min(img_size/img.size[0], img_size/img.size[1])
        imw = round(img.size[0] * ratio)
        imh = round(img.size[1] * ratio)
        img_transforms=transforms.Compose([transforms.Resize((imh,imw)),
                                           transforms.Pad((max(int((imh-imw)/2),0),
                                                           max(int((imw-imh)/2),0), max(int((imh-imw)/2),0),
                                                           max(int((imw-imh)/2),0)), (128,128,128)),
                                           transforms.ToTensor(),
        ])
        # convert image to Tensor
        image_tensor = img_transforms(img).float()
        image_tensor = image_tensor.unsqueeze_(0)
        input_img = Variable(image_tensor.type(self.tensor))
        # run inference on the model and get detections
        with torch.no_grad():
            detections = self.model(input_img)
            detections = utils.non_max_suppression(detections,
                                                   self.conf_thres, self.nms_thres)
        return detections[0]


    def process_detections(self, frame, detections):
        img = frame["image"]
        classes = self.classes
        scaled = []
        if detections is None:
            return

#        plt.figure()
#        fig, ax = plt.subplots(1)
#        fig.tight_layout(pad=0)
#        ax.imshow(img)

        if len(detections) > 0:
            #detections = utils.rescale_boxes(detections, 416, img.shape[:2])
            detections = utils.rescale_boxes(detections, 416, [frame["unscaled_height"], frame["unscaled_width"]])
            unique_labels = detections[:, -1].cpu().unique()
            n_cls_preds = len(unique_labels)

            # Get bounding-box colors
 #           cmap = plt.get_cmap('tab20b')
#            colors = [cmap(i) for i in np.linspace(0, 1, 20)]

#            bbox_colors = random.sample(colors, n_cls_preds)

            for x1, y1, x2, y2, conf, cls_conf, cls_pred in detections:
                self.logger.info("\t+ Label: %s, Conf: %.5f" % (classes[int(cls_pred)], cls_conf.item()))
                self.logger.info("\t+ (x: %s, y: %s) (%s, %s)" % (x1, y1, x2, y2))
                box_w = x2 - x1
                box_h = y2 - y1

                scaled.append([
                    frame["video_unit_id"],
                    frame["offset"],
                    'object',
                    classes[int(cls_pred)],
                    int(100 * cls_conf.item()),
                    int(x1.tolist()),
                    int(y1.tolist()),
                    int(x2.tolist()),
                    int(y2.tolist())
                ])
 #               color = bbox_colors[int(np.where(unique_labels == int(cls_pred))[0])]

                # Create rectangle patch
#                bbox = patches.Rectangle((x1, y1), box_w, box_h, linewidth=2, edgecolor=color, facecolor="none")
                # add box to the plot
#                ax.add_patch(bbox)
                # Add label
#                plt.text(
#                    x1,
#                    y1,
#                    s=classes[int(cls_pred)],
#             color="white",
#             verticalalignment="top",
#             bbox={"color": color, "pad": 0}
#             )


#        plt.axis("off")
#        plt.gca().xaxis.set_major_locator(NullLocator())
#        plt.gca().yaxis.set_major_locator(NullLocator())
#        fig.canvas.draw()
#        data = np.frombuffer(fig.canvas.tostring_rgb(), dtype=np.uint8)
#        data = data.reshape(fig.canvas.get_width_height()[::-1] + (3,))
#        plt.close(fig)

        if len(scaled) > 0:
            self.write_observations(scaled)

#        return data


    def handle_frame(self, frame):
        img_array = frame["image"]
        img = cv2.cvtColor(img_array, cv2.COLOR_BGR2RGB)
        img_pil = Image.fromarray(img)

        detections = self.detect_image(img_pil)
        img = img_array

        if detections is None:
            detections = []
        else:
            self.logger.info('Detection count: ' + str(len(detections)))
            data = self.process_detections(frame, detections)
#            self.write_frame('objects', data)


def main():
    worker = Yolo3Worker()
    worker.run()

if __name__ == "__main__":
    main()
