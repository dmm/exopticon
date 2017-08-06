// Brunch automatically concatenates all files in your
// watched paths. Those paths can be configured at
// config.paths.watched in "brunch-config.js".
//
// However, those files will only be executed if
// explicitly imported. The only exception are files
// in vendor, which are never wrapped in imports and
// therefore are always executed.

// Import dependencies
//
// If you no longer want to use a dependency, remember
// to also remove its path from "config.paths.watched".
import "phoenix_html";

// Import local files
//
// Local files can be imported directly using relative
// paths "./socket" or full ones "web/static/js/socket".

import socket from "./socket";

import CameraManager from "./camera_manager";

function updateCameras() {

    var request = new XMLHttpRequest();
    request.open("GET", "v1/cameras", true);

    request.onload = function() {
        if (this.status >= 200 && this.status < 400) {
            // Success!
            var cameras = JSON.parse(this.response);
            window.cameraManager.updateCameras(cameras);
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
    window.cameraManager = new CameraManager(socket);
    updateCameras();
    setInterval(updateCameras, 5000);
};
