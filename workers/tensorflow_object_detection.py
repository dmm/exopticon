#!/usr/bin/python3

import numpy as np
import os
import six.moves.urllib as urllib
import sys
import tarfile
import tensorflow as tf
import zipfile

from distutils.version import StrictVersion
from collections import defaultdict
from io import StringIO
from matplotlib import pyplot as plt
from PIL import Image

from exopticon_worker import ExopticonWorker

# This is needed since the notebook is stored in the object_detection folder.
from object_detection.utils import ops as utils_ops

if StrictVersion(tf.__version__) < StrictVersion('1.12.0'):
  raise ImportError('Please upgrade your TensorFlow installation to v1.12.*.')

from object_detection.utils import label_map_util

from object_detection.utils import visualization_utils as vis_util

# What model to download.
#MODEL_NAME = 'ssd_mobilenet_v2_coco_2018_03_29'
MODEL_NAME = 'faster_rcnn_inception_v2_coco_2018_01_28'
MODEL_FILE = MODEL_NAME + '.tar.gz'

# Path to frozen detection graph. This is the actual model that is used for the object detection.
PATH_TO_FROZEN_GRAPH = '/exopticon/workers/' + MODEL_NAME + '/frozen_inference_graph.pb'

# List of the strings that is used to add correct label for each box.
#PATH_TO_LABELS = os.path.join('data', 'mscoco_label_map.pbtxt')
PATH_TO_LABELS = '/exopticon/workers/' + 'mscoco_label_map.pbtxt'

def run_inference_for_single_image(image, sess):
  # Get handles to input and output tensors
  ops = tf.get_default_graph().get_operations()
  all_tensor_names = {output.name for op in ops for output in op.outputs}
  tensor_dict = {}
  for key in [
      'num_detections', 'detection_boxes', 'detection_scores',
      'detection_classes', 'detection_masks'
  ]:
    tensor_name = key + ':0'
    if tensor_name in all_tensor_names:
      tensor_dict[key] = tf.get_default_graph().get_tensor_by_name(
          tensor_name)
  if 'detection_masks' in tensor_dict:
    # The following processing is only for single image
    detection_boxes = tf.squeeze(tensor_dict['detection_boxes'], [0])
    detection_masks = tf.squeeze(tensor_dict['detection_masks'], [0])
    # Reframe is required to translate mask from box coordinates to image coordinates and fit the image size.
    real_num_detection = tf.cast(tensor_dict['num_detections'][0], tf.int32)
    detection_boxes = tf.slice(detection_boxes, [0, 0], [real_num_detection, -1])
    detection_masks = tf.slice(detection_masks, [0, 0, 0], [real_num_detection, -1, -1])
    detection_masks_reframed = utils_ops.reframe_box_masks_to_image_masks(
        detection_masks, detection_boxes, image.shape[1], image.shape[2])
    detection_masks_reframed = tf.cast(
        tf.greater(detection_masks_reframed, 0.5), tf.uint8)
    # Follow the convention by adding back the batch dimension
    tensor_dict['detection_masks'] = tf.expand_dims(
        detection_masks_reframed, 0)
  image_tensor = tf.get_default_graph().get_tensor_by_name('image_tensor:0')

  # Run inference
  output_dict = sess.run(tensor_dict,
                         feed_dict={image_tensor: image})

  # all outputs are float32 numpy arrays, so convert types as appropriate
  output_dict['num_detections'] = int(output_dict['num_detections'][0])
  output_dict['detection_classes'] = output_dict[
      'detection_classes'][0].astype(np.int64)
  output_dict['detection_boxes'] = output_dict['detection_boxes'][0]
  output_dict['detection_scores'] = output_dict['detection_scores'][0]
  if 'detection_masks' in output_dict:
    output_dict['detection_masks'] = output_dict['detection_masks'][0]

  return output_dict

def my_setup():
  # Load tensorflow model into memory
  detection_graph = tf.Graph()
  with detection_graph.as_default():
    od_graph_def = tf.GraphDef()
    with tf.gfile.GFile(PATH_TO_FROZEN_GRAPH, 'rb') as fid:
      serialized_graph = fid.read()
      od_graph_def.ParseFromString(serialized_graph)
      tf.import_graph_def(od_graph_def, name='')

  # Load category map
  category_index = label_map_util.create_category_index_from_labelmap(PATH_TO_LABELS, use_display_name=True)

  state = {}
  state["detection_graph"] = detection_graph
  state["category_index"] = category_index
  return state

def my_handle_frame(self, frame):
  # Expand dimensions since the model expects image to have shape: [1, None, None, 3]
  image_expanded = np.expand_dims(frame, axis=0)

  output_dict = run_inference_for_single_image(image_expanded,
                                               self.state['session'])
#  self.log_info('Detection count: ' + str(output_dict['num_detections']))

  vis_util.visualize_boxes_and_labels_on_image_array(
    frame,
    output_dict['detection_boxes'],
    output_dict['detection_classes'],
    output_dict['detection_scores'],
    self.state['category_index'],
    instance_masks=output_dict.get('detection_masks'),
    use_normalized_coordinates=True,
    line_thickness=8)
  self.write_frame('objects', frame)

  def my_cleanup(self):
    self.state['session'].close()

def main():
  import sys

  state = my_setup()
  config = tf.ConfigProto()
  config.gpu_options.per_process_gpu_memory_fraction = 0.1
  with state['detection_graph'].as_default():
    with tf.Session(config=config) as sess:
      state['session'] = sess
      worker = ExopticonWorker(handle_frame=my_handle_frame, state=state)

      worker.run()

if __name__ == "__main__":
    main()
