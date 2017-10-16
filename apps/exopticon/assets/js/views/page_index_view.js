import MainView from './main';
import CameraManager from '../camera_manager.js';
import socket from '../socket.js';

var $ = document.querySelect;
var $$ = document.querySelectAll;


export default class View extends MainView {
    updateCameras(cameraId) {
        var request = new XMLHttpRequest();
        request.open("GET", "/v1/cameras", true);

        request.onload = function() {
            if (this.status >= 200 && this.status < 400) {
                // Success!
                var cameras = JSON.parse(this.response);
                if (cameraId === undefined) {
                    window.cameraManager.updateCameras(cameras);
                } else {
                    cameras.forEach(function(c) {
                        if (c.id == cameraId) {
                            window.cameraManager.updateCameras([c]);
                        }
                    });
                }
            } else {
                console.log('reached server but something went wrong');
            }
        };

        request.onerror = function() {
            console.log('There was a connection error of some sort...');
        };

        request.send();
    }

    mount() {
        console.log('Mounting page index view.');
        window.cameraManager = new CameraManager(socket);
        this.updateCameras();
        setInterval(this.updateCameras, 5000);
    }
}
