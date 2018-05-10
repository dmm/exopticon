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

import PropTypes from 'prop-types';
import {use as jsJodaUse,
        DateTimeFormatter,
        Duration, ZonedDateTime, ZoneId} from 'js-joda';
import jsJodaTimeZone from 'js-joda-timezone';
import React from 'react';

import SuperImage from '../super_image';

import './../../css/components/file_browser.css';

jsJodaUse(jsJodaTimeZone);

/**
 * FileBrowser
 *
 */
class FileBrowser extends React.Component {
  /**
   * FileBrowser constructor
   * @param {object} props
   */
  constructor(props) {
    super(props);
    this.state = {
      files: props.initialFiles,
      selectedFile: undefined,
    };

    this.selectFile = this.selectFile.bind(this);
    this.playFile = this.playFile.bind(this);
    this.stop = this.stop.bind(this);
  }

  /**
   * formats a duration as a human readable string
   * @param {string} beginMonotonic - duration start
   * @param {string} endMonotonic - duration end
   * @return {string} formatting duration
   */
  formatDuration(beginMonotonic, endMonotonic) {
    let begin = parseInt(beginMonotonic, 10);
    let end = parseInt(endMonotonic, 10);

    let d = Duration.ofMillis((end - begin) / 1000);
    return d.toString();
  }

  /**
   * formats a date as a human readable string
   * @param {ZonedDateTime} date - input date to be formatted
   * @return {string} human readable datetime
   */
  formatDate(date) {
    let d = ZonedDateTime.parse(date);
    let ld = d.withZoneSameInstant(ZoneId.of(window.userTimezone));
    return ld.format(DateTimeFormatter.ofPattern('yyyy-MM-dd HH:mm:ss'));
  }

  /**
   * select file for playback
   * @param {Number} id - id of file to play
   *
   */
  selectFile(id) {
    this.setState({
      selectedFile: id,
    });
    this.playFile(id);
  }

  /**
   * @param {number} min
   * @param {number} max
   * @return {number} random number between min and max
   */
  getRandomInt(min, max) {
    min = Math.ceil(min);
    max = Math.floor(max);
    // The maximum is exclusive and the minimum is inclusive
    return Math.floor(Math.random() * (max - min)) + min;
  }

  /**
   * play specified file
   * @param {Number} fileId - id of file to play
  */
  playFile(fileId) {
    this.stop();
    let nonce = this.getRandomInt(0, 999999);
    this.topic = `playback:${nonce},${fileId},0`;
    this.channel = this.props.socket.channel(this.topic);

    this.channel.onError( (reason) =>
                          console.log('there was an error! ' + reason ));
    this.channel.onClose( () => {
      console.log('the channel has gone away gracefully');
    });

    this.superImage = new SuperImage(this._img);

    let self = this;
    this.channel.on('jpg', function(data) {
      self.channel.push('ack', {ts: data.ts});
      self.superImage.renderArrayIfReady(data.frameJpeg);
    });
    this.channel.join();
    this.channel.push('start_player', {topic: this.topic}, 10000);
  }

  /**
   * stops file playback
   *
   */
  stop() {
    if (this.channel) {
      this.channel.push('kill_player', {topic: this.topic});
      this.channel.leave();
      this.channel = undefined;
    }

    if (this.superImage) {
      this.superImage = undefined;
    }
  }

  /**
   * renders the component
   * @return {Object} react object
   */
  render() {
    const files = this.state.files.map((f) => {
      let classes = 'file-listing';
      if (this.state.selectedFile === f.id) {
        classes += ' selected';
      }
      return (
        <div className={classes}
             key={f.id}
             onClick={() => this.selectFile(f.id)}
             >
          <div>
            { this.formatDate(f.begin_time) }
          </div>
          <div>
            { this.formatDate(f.end_time) }
          </div>
          <div>
            { this.formatDuration(f.begin_monotonic, f.end_monotonic) }
          </div>
        </div>
      );
    });

    return (
      <div className='file-browser-wrapper'>
        <div className='file-list'>
          { files }
        </div>
        <div className='file-view'>
          <img ref={
                 (el) => {
                   this._img = el;
                 }
               }/>
        </div>
      </div>
    );
  }
}

FileBrowser.propTypes = {
  initialFiles: PropTypes.list,
  socket: PropTypes.object.isRequired,
};

FileBrowser.defaultProps = {
  initialFiles: [],
};

export default FileBrowser;
