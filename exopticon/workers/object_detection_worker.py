#!/usr/bin/python3

import copy
import cv2
import msgpack
import numpy
import struct
import sys
import time
from functools import partial
import os
import argparse
import logging
import cv2
import torch
import torchvision.transforms as tf
#import brambox.boxes as bbb
import lightnet as ln

from exopticon_worker import ExopticonWorker

#Parameters
CLASSES = 2
NETWORK_SIZE = [416, 416]
LABELS = ['person', 'squirrel']
CONF_THRESH = .25
NMS_THRESH = .1
LABELS = ['person', 'squirrel']

# Functions
def my_setup(self):
    self.log_info(os.getcwd())
    self.state['device'] = torch.device('cpu')
    if torch.cuda.is_available():
        self.state['device'] = torch.device('cuda')
    self.state['network'] = create_network(self.state['device'])

def create_network(device):
    """ Create the lightnet network """
    net = ln.models.TinyYolo(CLASSES, CONF_THRESH, NMS_THRESH)
    net.load("/exopticon/workers/yolov2-tiny-voc.weights")
    net.eval()
    net.postprocess.append(ln.data.transform.TensorToBrambox(NETWORK_SIZE, LABELS))
    net = net.to(device)
    return net


def detect(device, net, img):
    """ Perform a detection """
    im_h, im_w = img.shape[:2]

    img_tf = cv2.cvtColor(img, cv2.COLOR_BGR2RGB)
    img_tf = ln.data.transform.Letterbox.apply(img_tf, dimension=NETWORK_SIZE)
    img_tf = tf.ToTensor()(img_tf)
    img_tf.unsqueeze_(0)
    img_tf = img_tf.to(device)

    # Run detector
    with torch.no_grad():
        out = net(img_tf)
    reverse_letterbox = ReverseLetterbox2(NETWORK_SIZE, [im_w, im_h])
    out = reverse_letterbox(out)
    ###out = ln.data.transform.ReverseLetterbox.apply(out, NETWORK_SIZE, (im_w, im_h)) # Resize bb to true image dimensions

    return out

def my_handle_frame(self, frame):
    output = detect(self.state['device'], self.state['network'], frame)
    self.log_info("object count " + str(len(output[0])))
#    frame = bbb.draw_boxes(frame, output[0], show_labels=LABELS)
#    self.write_frame("objects", frame)

def main():
    worker = ExopticonWorker(setup=my_setup, handle_frame=my_handle_frame)
    worker.run()

if __name__ == "__main__":
    main()
