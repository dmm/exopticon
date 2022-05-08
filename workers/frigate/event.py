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


from collections import defaultdict
import cv2
import numpy as np
from scipy.spatial import distance as dist
import itertools
import random
import string
import json
import time
import uuid

from exopticon import ExopticonWorker

class TrackedObject(object):
    def __init__(self):
        pass
    
# borrowed from Frigate
class ObjectTracker():
    def __init__(self): #, config: DetectConfig):
        self.tracked_objects = {}
        self.disappeared = {}
        self.max_disappeared = 25

    def register(self, index, obj):
#        rand_id = ''.join(random.choices(string.ascii_lowercase + string.digits, k=6))
#        id = f"{obj['frame_time']}-{rand_id}"
        id = str(uuid.uuid4())
        obj['id'] = id
        obj['start_time'] = obj['frame_time']
        obj['last_seen'] = time.monotonic()
        obj['observations'] = [obj['observation']]
        self.tracked_objects[id] = obj
        self.disappeared[id] = 0

    def deregister(self, id):
        del self.tracked_objects[id]
        del self.disappeared[id]

    def update(self, id, new_obj):
        self.disappeared[id] = 0
        self.tracked_objects[id]['last_seen'] = time.monotonic()
        self.tracked_objects[id]['observations'].append(new_obj['observation'])
        self.tracked_objects[id].update(new_obj)

    def expire_old(self):
        t = time.monotonic()
        for obj in list(self.tracked_objects.values()):
            if t - obj['last_seen'] > 3:
                self.deregister(obj['id'])

    def match_and_update(self, frame_time, new_objects):
        # group by name
        new_object_groups = defaultdict(lambda: [])
        for obj in new_objects:
            new_object_groups[obj.details].append({
                'frame_time': frame_time,
                'label': obj.details,
                'score': obj.score,
                'box': obj.box(),
                'observation': obj
            })

        if len(new_objects) == 0:
            return

        # track objects for each label type
        for label, group in new_object_groups.items():
            current_objects = [o for o in self.tracked_objects.values() if o['label'] == label]
            current_ids = [o['id'] for o in current_objects]
            current_centroids = np.array([o['centroid'] for o in current_objects])

            # compute centroids of new objects
            for obj in group:
                centroid_x = int((obj['box'][0]+obj['box'][2]) / 2.0)
                centroid_y = int((obj['box'][1]+obj['box'][3]) / 2.0)
                obj['centroid'] = (centroid_x, centroid_y)

            if len(current_objects) == 0:
                for index, obj in enumerate(group):
                    self.register(index, obj)
                return

            new_centroids = np.array([o['centroid'] for o in group])

            # compute the distance between each pair of tracked
            # centroids and new centroids, respectively -- our
            # goal will be to match each new centroid to an existing
            # object centroid
            D = dist.cdist(current_centroids, new_centroids)

            # in order to perform this matching we must (1) find the
            # smallest value in each row and then (2) sort the row
            # indexes based on their minimum values so that the row
            # with the smallest value is at the *front* of the index
            # list
            rows = D.min(axis=1).argsort()

            # next, we perform a similar process on the columns by
            # finding the smallest value in each column and then
            # sorting using the previously computed row index list
            cols = D.argmin(axis=1)[rows]

            # in order to determine if we need to update, register,
            # or deregister an object we need to keep track of which
            # of the rows and column indexes we have already examined
            usedRows = set()
            usedCols = set()

            # loop over the combination of the (row, column) index
            # tuples
            for (row, col) in zip(rows, cols):
                # if we have already examined either the row or
                # column value before, ignore it
                if row in usedRows or col in usedCols:
                    continue

                # otherwise, grab the object ID for the current row,
                # set its new centroid, and reset the disappeared
                # counter
                objectID = current_ids[row]
                self.update(objectID, group[col])

                # indicate that we have examined each of the row and
                # column indexes, respectively
                usedRows.add(row)
                usedCols.add(col)

            # compute the column index we have NOT yet examined
            unusedRows = set(range(0, D.shape[0])).difference(usedRows)
            unusedCols = set(range(0, D.shape[1])).difference(usedCols)

            # in the event that the number of object centroids is
			# equal or greater than the number of input centroids
			# we need to check and see if some of these objects have
			# potentially disappeared
            # if D.shape[0] >= D.shape[1]:
            #     for row in unusedRows:
            #         id = current_ids[row]

            #         if self.disappeared[id] >= self.max_disappeared:
            #             self.deregister(id)
            #         else:
            #             self.disappeared[id] += 1
            # if the number of input centroids is greater
            # than the number of existing object centroids we need to
            # register each new input centroid as a trackable object
            #else:
            if D.shape[0] < D.shape[1]:
                for col in unusedCols:
                    self.register(col, group[col])

class EventWorker(ExopticonWorker):
    def __init__(self):
        super().__init__("EventWorker")
        self.tracked_objects = {}
        self.disappeared = {}
        self.max_disappeared = 30
        self.object_trackers = defaultdict(lambda: ObjectTracker())

    def handle_frame(self, frame):
        image = frame.image
        offset = [0, 0]
        object_tracker = self.object_trackers[frame.camera_id]

        object_tracker.expire_old()
        object_tracker.match_and_update(frame.analysis_offset, frame.observations)
        self.logger.debug("Incoming detections: " + str(len(frame.observations)))
#        self.logger.info("Tracking " + str(len(object_tracker.tracked_objects)) + " Objects: " + json.dumps(object_tracker.tracked_objects))

        for obj in object_tracker.tracked_objects.values():
            count = len(obj['observations'])
            sum = 0
            max_score = 0
            max_id = 0
            for obs in obj['observations']:
                if obs.score > max_id:
                    max_score = obs.score
                    max_id = obs.id
                sum += obs.score
            if object_tracker.disappeared[obj['id']] == 0 and (sum / count) > 80 and count > 3:
                obj["camera_id"] = frame.camera_id
                obj["display_observation_id"] = max_id
                self.write_event(obj)

    def handle_timeout(self):
        for object_tracker in list(self.object_trackers.values()):
            object_tracker.expire_old()

if __name__ == "__main__":
    worker = EventWorker()
    worker.run()
