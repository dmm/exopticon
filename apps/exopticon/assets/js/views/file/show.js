/*
 * This file is part of Exopticon (https://github.com/dmm/exopticon).
 * Copyright (c) 2018 David Matthew Mattli
 *
 * Exopticon is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Exopticon is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Exopticon.  If not, see <http://www.gnu.org/licenses/>.
 */

import MainView from '../main';
import socket from '../../socket';

/**
 * @param {number} min
 * @param {number} max
 * @return {number} random number between min and max
 */
function getRandomInt(min, max) {
  min = Math.ceil(min);
  max = Math.floor(max);
  // The maximum is exclusive and the minimum is inclusive
  return Math.floor(Math.random() * (max - min)) + min;
}

/**
 * FileShow view class
 * @class
 */
export default class View extends MainView {
  /**
   * view entry point
   */
  mount() {
    super.mount();

    // Specific logic here
    console.log('ShowFileView mounted');
    let fileId = document.getElementById('fileId').textContent;
    let nonce = getRandomInt(0, 999999);
    const topic = `playback:${nonce},${fileId},0`;
    let channel = socket.channel(topic);
    channel.onError( (reason) => console.log('there was an error! ' + reason ));
    channel.onClose( () => {
      console.log('the channel has gone away gracefully');
    });
    let videoDiv = document.querySelector('.video');
    let img = document.createElement('img');
    videoDiv.appendChild(img);
    channel.on('jpg', function(data) {
      //          renderFrame(img, data.frameJpeg);
    });
    channel.join();
    channel.push('start_player', {topic: topic}, 10000);
    window.chan1 = channel;
  }

  /**
   * view exit callback
   */
  unmount() {
    super.unmount();
  }
}
