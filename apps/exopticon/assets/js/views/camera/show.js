import MainView from '../main';

import CameraManager from '../../camera_manager.js';
import socket from '../../socket';
import renderFrame from '../../render_frame.js';

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

    mount() {
        super.mount();
        console.log('ShowCameraView mounted.');

        let cameraId = parseInt(document.getElementById('singleCamera').getAttribute('data-id'), 10);
        window.cameraManager = new CameraManager(socket);
        this.fetchCamera(cameraId);
        this.fetchFiles();
    }

    unmount() {

    }
}
