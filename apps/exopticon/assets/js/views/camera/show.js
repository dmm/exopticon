'use strict';



import React from 'react';
import ReactDOM from 'react-dom';
import CameraOverlay from '../../components/camera_overlay';
import CameraPlayer from '../../camera_player';
import CameraView from '../../components/camera_view';
import CameraPanel from '../../components/camera_panel';
import MainView from '../main';
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

  fetchFiles(cameraId, beginTime, endTime) {
    var request = new XMLHttpRequest();
    request.open('GET', `/v1/files?cameraId=${cameraId}&beginTime=${beginTime}&endTime=${endTime}`, true);
    request.onload = function() {
      if (this.status >= 200 && this.status < 400) {
        // Success!
        var files = JSON.parse(this.response);
      } else {
        console.log('reached server but something went wrong');
      }
    };

    request.onerror = function() {
      console.log('There was a connection error of some sort...');
    };

    request.send();

  }
  updateCameras(cameraId) {
    var self = this;
    fetch('/v1/cameras', {
      credentials: 'same-origin',
      headers: {
        'Content-Type': 'application/json'
      }
    }).then((response) => {
      return response.json();
    }).then((cameras) => {

      cameras.forEach((c) => {
        if (c.id == cameraId) {
          self.panelComponent.setState({cameras: [c]});
        }
      });
    });
  }

  mount() {
    super.mount();
    console.log('ShowCameraView mounted.');

    let cameraId = parseInt(document.getElementById('singleCamera').getAttribute('data-id'), 10);
    this.panel = React.createElement(CameraPanel,
                                     {
                                       socket: socket,
                                       initialCameras: new Map()
                                     });
    this.panelComponent = ReactDOM.render(this.panel, document.getElementById('main'));
    this.updateCameras(cameraId);
  }

  unmount() {
    super.unmount();
  }
}
