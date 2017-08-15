
import socket from "./socket";

import CameraManager from "./camera_manager";

function updateCameras(cameraId) {
    var request = new XMLHttpRequest();
    request.open("GET", "v1/cameras", true);

    request.onload = function() {
        if (this.status >= 200 && this.status < 400) {
            // Success!
            var cameras = JSON.parse(this.response);
            cameras.forEach(function (c) {
                if (c.id === cameraId) {
                    window.cameraManager.updateCameras(cameras);
                }
            });
        } else {
            console.log('reached server but something went wrong');
        }
    };

    request.onerror = function() {
        console.log('There was a connection error of some sort...');
    };

    request.send();
}

window.onload = function() {
    let cam = document.getElementById("singleCamera");
    if (cam) {
        window.cameraManager = new CameraManager(socket);
        updateCameras(cam.dataset.id);
        setInterval(updateCameras, 5000);
    }
};
