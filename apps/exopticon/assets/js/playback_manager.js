
import renderFrame from './render_frame.js';

function getRandomInt(min, max) {
    min = Math.ceil(min);
    max = Math.floor(max);
    return Math.floor(Math.random() * (max - min)) + min; //The maximum is exclusive and the minimum is inclusive
}

var PlaybackManager = function(socket, img, files) {
    this.socket = socket;
    this.img = img;
    this.channel = null;
    this.files = files;
};

PlaybackManager.prototype.playFile = function(fileId, msOffset) {
    const nonce = getRandomInt(0, 999999);
    const topic = `playback:${nonce},${fileId},${msOffset}`;
    let channel = this.socket.channel(topic);
    channel.onError( (reason) => console.log("there was an error! " + reason ));
    channel.onClose( () => console.log("the channel has gone away gracefully") );
    channel.on('jpg', (data) => {
        renderFrame(this.img, data.frameJpeg);
    });
    channel.on('stop', (data) => {
        console.log('video stop!');
    });
    channel.join();
    channel.push('start_player', { topic: topic }, 10000);
    this.channel = channel;
    this.topic = topic;
};

PlaybackManager.prototype.stopFile = function() {
    if (this.channel !== null) {
        this.channel.push('kill_player', { topic: this.topic }, 10000);
    }
};

PlaybackManager.prototype.play = function() {
    let file = this.files[0];
    this.playFile(file.id, 0);
};

export default PlaybackManager;
