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


import {
  use as jsJodaUse,
  DateTimeFormatter,
  LocalDateTime,
  Duration,
  ZonedDateTime,
  ZoneOffset,
} from 'js-joda';
import jsJodaTimeZone from 'js-joda-timezone';
import PropTypes from 'prop-types';
import React from 'react';

import FileLibrary from '../file_library';
import FilePlayer from '../file_player';
import ProgressBar from './progress_bar';

import '../../css/components/camera_player_view.css';

class CameraPlayerView extends React.Component {
  constructor(props) {
    super(props);

    this.state = {
      status: 'loading',
    };

    this.fileLibrary = new FileLibrary(this.props.videoUnits);
    this.filePlayer = undefined;
    this.timeSelect = this.timeSelect.bind(this);

  }

  timeSelect(time) {
    if (this.filePlayer) {
      this.filePlayer.stop();
      this.filePlayer = undefined;
    }

    console.log('Selected: ' + time.toString());
    let ret = this.fileLibrary.getFileForTime(time);
    this.filePlayer = new FilePlayer(ret.file.files[0].id, ret.offset, this._img);
    console.log(ret);
  }

  /**
   * react render function
   * @return {Object} react entity
   */
  render() {
    return (
      <div
           className="camera-player-view"
        >
        <div className="player-width"></div>
        <div className="content">
          <div className="wrapper">
            <div className="frame">
              <img ref={
                     (el) => {
                       this._img = el;
                     }
                }/>
            </div>
          <ProgressBar
            beginTime={this.props.beginTime}
            endTime={this.props.endTime}
            videoUnits={this.props.videoUnits}
            onClick={this.timeSelect}
            />
          </div>
        </div>
      </div>
    );
  }
}

export default CameraPlayerView;
