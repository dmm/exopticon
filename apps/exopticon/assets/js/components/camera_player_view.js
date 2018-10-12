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
  DateTimeFormatter,
  Duration,
  ZoneId,
} from 'js-joda';
import PropTypes from 'prop-types';
import React from 'react';

import FileLibrary from '../file_library';
import FilePlayer from '../file_player';

import OverlayButton from './overlay_button';
import ProgressBar from './progress_bar';
import StatusOverlay from './status_overlay';

import '../../css/components/camera_player_view.css';

/**
 * CameraPlayerView: implements video recording player for a camera
 * @class
 */
class CameraPlayerView extends React.Component {
  /**
   * CameraPlayerView constructor
   * @param {Object} props
*/
  constructor(props) {
    super(props);

    this.state = {
      status: 'stopped',
      beginTime: this.props.initialBeginTime,
      endTime: this.props.initialEndTime,
      library: undefined,
      currentVideoUnit: undefined,
      progress: 0,
    };

    this.filePlayer = undefined;

    this.setInterval(this.props.initialBeginTime, this.props.initialEndTime);

    // Bind callbacks
    this.timeSelect = this.timeSelect.bind(this);
    this.showTime = this.showTime.bind(this);
    this.hideTime = this.hideTime.bind(this);
    this.playFile = this.playFile.bind(this);
    this.play = this.play.bind(this);
    this.pause = this.pause.bind(this);
    this.setInteval = this.setInterval.bind(this);
    this.shiftInterval = this.shiftInterval.bind(this);
    this.formatLocal = this.formatLocal.bind(this);
  }

  /**
   * Changes display interval
   * @param {ZonedDateTime} beginTime - beginning of interval
   * @param {ZonedDateTime} endTime - end of interval
   */
  setInterval(beginTime, endTime) {
    if (this.filePlayer) {
      this.filePlayer.stop();
      this.filePlayer = undefined;
    }

    this.props.videoUnitRepository.fetchBetween(this.props.cameraId,
                                                beginTime,
                                                endTime)
      .then((units) => {
        let library = new FileLibrary(units);
        this.setState({
          beginTime: beginTime,
          currentVideoUnit: undefined,
          endTime: endTime,
          library: library,
          progress: 0,
          progressHover: false,
        });
      });
  }

  /**
   * Change interval by given number of minutes
   * @param {Number} minutes - shift amount
   */
  shiftInterval(minutes) {
    const newBegin = this.state.beginTime.plusMinutes(minutes);
    const newEnd = this.state.endTime.plusMinutes(minutes);

    this.setState({
      status: 'loading',
    });

    this.setInterval(newBegin, newEnd);
  }

  /**
   * Format given time as a local time
   * @param {ZonedDateTime} time - time to format
   * @return {string} - formatted local date time
   */
  formatLocal(time) {
    const localTime = time.withZoneSameInstant(ZoneId.of(this.props.timezone));
    return localTime.format(DateTimeFormatter.ofPattern('yyyy-MM-dd HH:mm:ss'));
  }

  /**
   * Begin playback of a given file
   * @param {Number} fileId - id of file to play
   * @param {Number} offset - offset in milliseconds to begin playback from
   * @param {Image} img - image to render video to
   *
   */
  playFile(fileId, offset, img) {
    if (this.filePlayer) {
      this.filePlayer.stop();
      this.filePlayer = undefined;
    }

    this.filePlayer = new FilePlayer(fileId, offset, img, (state) => {
      if (state.status === 'started') {
        const duration = Duration.between(this.state.beginTime,
                                         this.state.endTime).toMillis();
        let unit = this.state.currentVideoUnit;
        const playbackTime = unit.begin_time.plusNanos(state.offset * 1000000);
        const offset = Duration.between(this.state.beginTime,
                                        playbackTime).toMillis();

        this.setState({
          status: state.status,
          progress: offset / duration * 100,
          fileOffset: state.offset,
          currentPlaybackTime: playbackTime,
        });
      } else if (state.status === 'stopped') {
        const nextUnit = this.state.library.getNextFile(fileId);
        console.log(nextUnit);
        if (nextUnit !== null) {
          this.setState({
            currentVideoUnit: nextUnit,
          });
          if (this.filePlayer) {
            this.filePlayer.stop();
            this.filePlayer = undefined;
          }
          this.playFile(nextUnit.files[0].id, 0, img);
        }
      }
    });
  }

  /**
   * Resume playback or begin from start of current interval
   */
  play() {
    if (this.state.currentVideoUnit) {
      this.playFile(this.state.currentVideoUnit.files[0].id,
                    this.state.fileOffset,
                    this._img);
    } else if (this.state.library.files.length > 0) {
      this.timeSelect(this.state.beginTime);
    }
  }

  /**
   * Pause playback
   */
  pause() {
    if (this.filePlayer) {
      this.filePlayer.stop();
      this.filePlayer = undefined;
    }

    this.setState({
      status: 'paused',
    });
  }

  /**
   * Begin playback at specified time.
   * @param {ZonedDateTime} time - time to begin playback
   */
  timeSelect(time) {
    console.log('Selected: ' + time.toString());
    let ret = this.state.library.getFileForTime(time);
    if (ret) {
      this.setState({
        currentVideoUnit: ret.file,
        status: 'firstLoad',
      });
      this.playFile(ret.file.files[0].id, ret.offset, this._img);
    }
  }

  /**
   * Show time hover indicator
   */
  showTime() {
    this.setState({
      progressHover: true,
    });
  }

  /**
   * Hide time indicator
   */
  hideTime() {
    this.setState({
      progressHover: false,
    });
  }

  /**
   * react render function
   * @return {Object} react entity
   */
  render() {
    let status;

    if (this.state.status === 'firstLoad') {
      status = (
        <StatusOverlay status='loading' cameraName='video' />
      );
    }

    if (this.state.library === undefined) {
      return (
        <div></div>
      );
    }

    let displayTime = '';
    if (this.state.status === 'started' || this.state.status === 'paused') {
      displayTime = this.formatLocal(this.state.currentPlaybackTime);
    }

    let control = (<OverlayButton
                   onClick={ this.play }
                   label="▶" />);

    if (this.state.status === 'started') {
      control = (<OverlayButton
                 onClick={ this.pause }
                 label="⏸" />);
    }

    const videoUnits = this.state.library.files;

    return (
      <div
        className="camera-player-view"
        >
        <div className="wrapper">
          <div className="frame">
            { status }
            <img ref={
                   (el) => {
                     this._img = el;
                   }
              }/>
          </div>
          <div className='control-bar'>
            <div className='begin-time'>
              {this.formatLocal(this.state.beginTime)}
            </div>
            <div className='time-label'>
              <span>{displayTime}</span>
            </div>
            <div className='controls'>
              { control }
            </div>
          </div>
          <div className='tool-wrapper'>
            <OverlayButton
              label='◀'
              onClick={
                ()=> {
                  this.shiftInterval(-30);
                }
              }
              />
              <ProgressBar
                beginTime={this.state.beginTime}
                endTime={this.state.endTime}
                videoUnits={videoUnits}
                onClick={this.timeSelect}
                onTimeHover={this.showTime}
                onTimeLeave={this.hideTime}
                progress={this.state.progress}
                formatLocal={this.formatLocal}
                  />
              <OverlayButton
                label='▶'
                onClick={
                  () => {
                    this.shiftInterval(30);
                  }
                }
                />
            </div>
        </div>
      </div>
    );
  }
}
CameraPlayerView.propTypes = {
  cameraId: PropTypes.number.isRequired,
  initialBeginTime: PropTypes.object.isRequired,
  initialEndTime: PropTypes.object.isRequired,
  timezone: PropTypes.string,
  videoUnitRepository: PropTypes.object.isRequired,
};

CameraPlayerView.defaultProps = {
  timezone: 'Etc/UTC',
};

export default CameraPlayerView;
