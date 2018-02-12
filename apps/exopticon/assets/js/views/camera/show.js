import { use as jsJodaUse, ZonedDateTime, ZoneOffset } from 'js-joda';
import jsJodaTimeZone from 'js-joda-timezone';
import React from 'react';
import ReactDOM from 'react-dom';

import CameraManager from '../../camera_manager.js';
import MainView from '../main';
import ProgressBar from '../../components/progress_bar';
import socket from '../../socket';

export default class view extends MainView {
  fetchCamera(cameraId) {
    var request = new XMLHttpRequest();
    request.open('GET', '/v1/cameras/' + cameraId, true);
    request.onload = function() {
      if (this.status >= 200 && this.status < 400) {
        // Success!
        var camera = JSON.parse(this.response);
        window.cameraManager.updateCameras([camera]);
      } else {
        console.log('reached server but something went wrong');
      }
    };

    request.onerror = function() {
      console.log('There was a connection error of some sort...');
    };

    request.send();
  }

  fetchFiles(cameraId, beginTime, endTime, progress) {
    fetch(`/v1/files/${cameraId}?begin_time=${beginTime}&end_time=${endTime}`, {
      credentials: 'same-origin',
      headers: {
        'Content-Type': 'application/json'
      }
    }).then((response) => {
      return response.json();
    }).then((files) => {
      console.log('Got ' + files.length + ' files. Setting state...');
      progress.setState({ files: files });
    }).catch((error) => {
      console.log('There was an error fetching files: ' + error);
    });
  }

  mount() {
    super.mount();
    console.log('ShowCameraView mounted.');

    let cameraId = parseInt(document.getElementById('singleCamera').getAttribute('data-id'), 10);
    window.cameraManager = new CameraManager(socket);
    this.fetchCamera(cameraId);
    const now = ZonedDateTime.now(ZoneOffset.UTC);
    const then = now.minusHours(12);

    var progressBar = React.createElement(ProgressBar,
                                          {

                                          });
    this.progressComponent = ReactDOM.render(progressBar, document.getElementById('progress'));
    this.fetchFiles(cameraId, then.toString(), now.toString(), this.progressComponent);
  }

  unmount() {

  }
}
