#!/usr/bin/python3

import copy
import cv2
import msgpack
import numpy
import struct
import sys
import time
from functools import partial
import imutils

from exopticon_worker import ExopticonWorker

def my_setup(self):
#    fgbg = cv2.createBackgroundSubtractorKNN()
#    fgbg = cv2.bgsegm.createBackgroundSubtractorCNT()
#    fgbg = cv2.createBackgroundSubtractorMOG2()
    fgbg = cv2.bgsegm.createBackgroundSubtractorGMG()
#    fgbg.setDetectShadows(True)
    self.state['fgbg'] = fgbg

def my_handle_frame(self, frame):
    frame = imutils.resize(frame["image"], width=400)
    gray = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)
#    gray = cv2.GaussianBlur(frame, (21, 21), 0)

    fgmask = self.state['fgbg'].apply(gray)
    #self.write_frame("foreground", fgmask)

    # dilate the thresholded image to fill in holes then find contours
    thresh = cv2.dilate(fgmask, None, iterations=2)
    cnts = cv2.findContours(thresh.copy(), cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
    cnts = imutils.grab_contours(cnts)

    # loop over contours
    for c in cnts:
        # ignore small contours
        if cv2.contourArea(c) < 50: #50
            continue
        # compute the bounding box and draw it on frame
        (x, y, w, h) = cv2.boundingRect(c)
        cv2.rectangle(frame, (x, y), (x+w, y+h), (0, 255, 0), 2)

    self.write_frame("contours", frame)

def main():
    worker = ExopticonWorker(setup=my_setup, handle_frame=my_handle_frame)
    worker.run()

if __name__ == "__main__":
    main()
