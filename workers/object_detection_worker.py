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

class ExopticonWorker(object):
    def __init__(self, handle_frame=None, setup=None):
        self.handle_frame_callback = self.builtin_handle_frame_callback
        self.setup_callback = self.builtin_setup_callback

        if handle_frame:
            self.handle_frame_callback = partial(handle_frame, self)
        if setup:
            self.setup_callback = partial(setup, self)

    def builtin_setup_callback(self):
        self.state = {}

    def builtin_handle_frame_callback(self, frame):
        log_info('frame size' + str(frame.shape))

    def setup(self):
        self.state = {}
        self.setup_callback()

    def cleanup(self):
        self.log_info("cleaning up!")

    def handle_frame(self, frame):
        start_time = time.monotonic()
        self.handle_frame_callback(frame)
        duration = time.monotonic() - start_time
        self.log_info('Ran for :' + str(duration * 1000) + ' ms')

    def log_info(self, message):
        log_dict = [0, [message]]
        serialized = msgpack.packb(log_dict, use_bin_type=True)
        self.write_framed_message(serialized)

    def request_frame(self):
        request = [1, [1]]
        serialized = msgpack.packb(request, use_bin_type=True)
        self.write_framed_message(serialized)

    def read_frame(self):
        len_buf = sys.stdin.buffer.read(4)
        msg_len = struct.unpack('>L', len_buf)[0]
        msg_buf = sys.stdin.buffer.read(msg_len)
        msg = msgpack.unpackb(msg_buf, raw=False)
        self.current_frame = msg[1][0]
        msg_buf = numpy.frombuffer(msg[1][0]["jpeg"], dtype=numpy.uint8)
        return cv2.imdecode(msg_buf, cv2.IMREAD_UNCHANGED)

    def write_frame(self, tag, image):
        if not self.current_frame:
            return
        frame = copy.copy(self.current_frame)
        jpeg = cv2.imencode('.jpg', image)[1].tobytes()
        frame_dict = [3, [tag, jpeg]]
        serialized = msgpack.packb(frame_dict, use_bin_type=True)
        self.write_framed_message(serialized)

    def write_framed_message(self, serialized):
        packed_len = struct.pack('>L', len(serialized))
        sys.stdout.buffer.write(packed_len)
        sys.stdout.buffer.write(serialized)
        sys.stdout.buffer.flush()

    def run(self):
        self.setup()
        try:
            while True:
                self.request_frame()
                frame = self.read_frame()
                self.handle_frame(frame)
        except EOFerror:
            self.cleanup()
            sys.exit(0)
# End ExopticonWorker


class ReverseLetterbox2(ln.data.transform.util.BaseTransform):
    """ Performs a reverse letterbox operation on the bounding boxes, so they can be visualised on the original image.
    Args:
        network_size (tuple): Tuple containing the width and height of the images going in the network
        image_size (tuple): Tuple containing the width and height of the original images
    Returns:
        (list [list [brambox.boxes.Detection]]): list of brambox detections per image
    Note:
        This transform works on :class:`brambox.boxes.Detection` objects,
        so you need to apply the :class:`~lightnet.data.TensorToBrambox` transform first.
    Note:
        Just like everything in PyTorch, this transform only works on batches of images.
        This means you need to wrap your tensor of detections in a list if you want to run this transform on a single image.
    """
    def __init__(self, network_size, image_size):
        self.network_size = network_size
        self.image_size = image_size
    def __call__(self, boxes):
        im_w, im_h = image_size[:2]
        net_w, net_h = self.network_size[:2]
        if im_w == net_w and im_h == net_h:
            scale = 1
        elif im_w / net_w >= im_h / net_h:
            scale = im_w/net_w
        else:
            scale = im_h/net_h
        pad = int((net_w - im_w/scale) / 2), int((net_h - im_h/scale) / 2)
        converted_boxes = []
        for b in boxes:
            converted_boxes.append(self._transform(b, scale, pad))
        return converted_boxes
    @staticmethod
    def _transform(boxes, scale, pad):
        for box in boxes:
            box.x_top_left -= pad[0]
            box.y_top_left -= pad[1]
            box.x_top_left *= scale
            box.y_top_left *= scale
            box.width *= scale
            box.height *= scale
        return boxes

#Parameters
CLASSES = 1
NETWORK_SIZE = [416, 416]
LABELS = ['person']
CONF_THRESH = .25
NMS_THRESH = .1
LABELS = ['person']

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
