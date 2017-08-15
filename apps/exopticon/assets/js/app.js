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

import SingleCamera from "./singleCamera";

function updateCameras(cameraId) {

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

function fetchFiles(cameraId) {
    var request = new XMLHttpRequest();
    request.open("GET", "/v1/files/" + cameraId, true);

    request.onload = function() {
        setTimeSelector(processFiles(JSON.parse(this.response)));
    };

    request.onerror = function() {
        console.log('There was a problem with the file connection...');
    };

    request.send();
}

function processFiles(files) {
    let length = 0;

    files.forEach(function (f) {
        length += f.end_monotonic - f.begin_monotonic;
    });

    return {
        length: length / 1000
    };
}

function setTimeSelector(processedFiles) {
    let s = document.getElementById('timeSelector');

    s.min = 0;
    s.max = processedFiles.length;
}

window.onload = function() {
    if (document.getElementById("allCameras")) {
        window.cameraManager = new CameraManager(socket);
        updateCameras();
        setInterval(updateCameras, 5000);
    }

    let cam = document.getElementById("singleCamera");
    if (cam) {
        window.cameraManager = new CameraManager(socket);
        updateCameras(cam.dataset.id);
        fetchFiles(cam.dataset.id);
    }
};
