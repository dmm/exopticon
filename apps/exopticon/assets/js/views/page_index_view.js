import MainView from './main';
import CameraManager from '../camera_manager.js';
import socket from '../socket.js';

var $ = document.querySelect;
var $$ = document.querySelectAll;

export default class View extends MainView {
  updateCameras() {
    fetch('/v1/cameras', {
      credentials: 'same-origin',
      headers: {
        'Content-Type': 'application/json'
      }
    }).then((response) => {
      return response.json();
    }).then((cameras) => {
      window.cameraManager.updateCameras(cameras);
    });
  }

  mount() {
    console.log('Mounting page index view.');
    window.cameraManager = new CameraManager(socket);
    this.updateCameras();

    document.addEventListener('visibilitychange', function() {
      console.log('visibility change!');
    });
  }
}
