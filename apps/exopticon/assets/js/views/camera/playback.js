import MainView from '../main';

import socket from '../../socket';

import PlaybackManager from '../../playback_manager.js';

let $ = (q) => { return document.querySelector(q); };

function fetchVideos(cameraId, beginTime, endTime, callback) {
    var request = new XMLHttpRequest();
    request.open('GET',
                 `/v1/files/${cameraId}?begin_time=${beginTime}&end_time=${endTime}`);
    request.onload = function() {
        if (this.status >= 200 && this.status < 400) {
            var videos = JSON.parse(this.response);
            callback(videos);
        } else {
            console.log('reached server but something went wrong.');
        }
    };

    request.onerror = function() {
        console.log('There was a connection error of some sort...');
    };

    request.send();
}

export default class View extends MainView {
    mount() {
        super.mount();
        console.log('mounted playback!');
        let playbackManager = null;

        let cameraId = $('#singleCamera').attributes['data-id'].value;
        let endTime = new Date();
        let beginTime = new Date();
        beginTime.setHours(beginTime.getHours() - 12);
        console.log('Fetching video from ' + beginTime.toISOString() + ' to ' +
                    endTime.toISOString());
        fetchVideos(cameraId, beginTime.toISOString(), endTime.toISOString(),
                    function(videos) {
                        console.log(videos[0]);
                        playbackManager = new PlaybackManager(socket, document.getElementById('playback-img'), videos);
                        playbackManager.play(); 
                    });
    }

    unmount() {

    }
}
