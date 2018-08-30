/*
 * This file is a part of Exopticon, a free video surveillance tool. Visit
 * https://exopticon.org for more information.
 *
 * Copyright (C) 2018 David Matthew Mattli
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

'use strict';

import SuperImage from './super_image';
import socket from './socket';

class FilePlayer {
  constructor(fileId, offset, img, statusCallback = ()=>{}) {
    this.fileId = fileId;
    this.offset = offset;
    this.img = img;
    this.statusCallback = statusCallback;

    let superImage = new SuperImage(img);

    let nonce = this.getRandomInt(0, 999999);
    this.topic = `playback:${nonce},${fileId},${offset}`;
    this.channel = socket.channel(this.topic);

    this.channel.onError( (reason) => console.log('there was an error! ' + reason ));
    this.channel.onClose( () => {
      console.log('the channel has gone away gracefully');
    });

    this.channel.on('jpg', (data) =>{
      this.channel.push('ack', {ts: data.ts});
      superImage.renderArrayIfReady(data.frameJpeg);
    });
    this.channel.join();
    this.channel.push('start_player', {topic: this.topic}, 10000);
  }

  stop() {
    this.channel.push('kill_player', {topic: this.topic}, 10000);
    this.channel.leave();
  }

  getRandomInt(min, max) {
    min = Math.ceil(min);
    max = Math.floor(max);
    // The maximum is exclusive and the minimum is inclusive
    return Math.floor(Math.random() * (max - min)) + min;
  }
}


export default FilePlayer;
