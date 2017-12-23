import MainView from '../main';

import socket from '../../socket';
//import '../../camera_manager.js' as CameraManager;
import renderFrame from '../../render_frame.js';

function getRandomInt(min, max) {
    min = Math.ceil(min);
    max = Math.floor(max);
    return Math.floor(Math.random() * (max - min)) + min; //The maximum is exclusive and the minimum is inclusive
}

export default class View extends MainView {

  mount() {
    super.mount();

    // Specific logic here
      console.log('ShowFileView mounted');
      var fileId = document.getElementById("fileId").textContent;
      var nonce = getRandomInt(0, 999999);
      const topic = `playback:${nonce},${fileId},0`;
      let channel = socket.channel(topic);
      channel.onError( (reason) => console.log("there was an error! " + reason ));
      channel.onClose( () => console.log("the channel has gone away gracefully") );
      let videoDiv = document.querySelector('.video');
      let img = document.createElement('img');
      videoDiv.appendChild(img);
      channel.on('jpg', function(data) {
          renderFrame(img, data.frameJpeg);
      });
      channel.join();
      channel.push("start_player", { topic: topic }, 10000);
      window.chan1 = channel;
  }

  unmount() {
    super.unmount();

    // Specific logic here
    console.log('ShowFileView unmounted');
  }
}
