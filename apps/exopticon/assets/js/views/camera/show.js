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

import {
  use as jsJodaUse,
  DateTimeFormatter,
  LocalDateTime,
  Duration,
  ZonedDateTime,
  ZoneOffset,
} from 'js-joda';
import jsJodaTimeZone from 'js-joda-timezone';

import React from 'react';
import ReactDOM from 'react-dom';

import FileLibrary from '../../file_library';
import MainView from '../main';
import CameraPlayerView from '../../components/camera_player_view';
import socket from '../../socket';

jsJodaUse(jsJodaTimeZone);

/**
 * CameraShow class
 * Implements page that shows a single camera
 */
export default class view extends MainView {
  /**
   * CameraShow constructor
   */
  constructor() {
    super();
  }

  /**
   * fetches camera details
   * @param {number} cameraId
   */
  fetchCamera(cameraId) {
    let request = new XMLHttpRequest();
    request.open('GET', '/v1/cameras/' + cameraId, true);
    request.onload = function() {
      if (this.status >= 200 && this.status < 400) {
        // Success!
        let camera = JSON.parse(this.response);
        window.cameraManager.updateCameras([camera]);
      } else {
        console.log('reached server but something went wrong');
      }
    };

    request.onerror = function() {
      console.log('There was a connection error of some sort...');
    };

    request.send();
  }

  /**
   * fetches video_units for interval
   * @param {Number} cameraId
   * @param {ZonedDateTime} beginTime
   * @param {ZonedDateTime} endTime
   *
   */
  fetchAvailability(cameraId, beginTime, endTime) {
    let request = new XMLHttpRequest();
    request.open('GET', `/v1/video_units/between?camera_id=${cameraId}&begin_time=${beginTime.toString()}&end_time=${endTime.toString()}`, true);
    request.onload = function() {
      if (this.status >= 200 && this.status < 400) {
        // Success!
      } else {
        console.log('reached server but something went wrong');
      }
    };

    request.onerror = function() {
      console.log('There was a connection error of some sort...');
    };

    request.send();
  }

  /**
   * page entry point
   */
  mount() {
    super.mount();
    console.log('ShowCameraView mounted.');

    let cameraId = parseInt(document.getElementById('singleCamera')
                            .getAttribute('data-id'), 10);

    const now = ZonedDateTime.now(ZoneOffset.UTC);
    const begin = now.minus(Duration.ofHours(1))
          .minus(Duration.ofMinutes(now.minute())
                 .plusSeconds(now.second())
                 .plusNanos(now.nano()))
          .plus(Duration.ofNanos(1));

    const end = begin.plus(Duration.ofHours(1));
        console.log('Now: ' + now.toString());
    console.log('Begin: ' + begin.toString());
    console.log('End: ' + end.toString());
    fetch(`/v1/video_units/between?camera_id=${cameraId}&begin_time=${begin.toString()}&end_time=${end.toString()}`, {
      credentials: 'same-origin',
      headers: {
        'Content-Type': 'application/json',
      },
    }).then((response) => {
      return response.json();
    }).then((videoUnits) => {
      let playerView = React.createElement(CameraPlayerView,
                                           {
                                             beginTime: begin,
                                             endTime: end,
                                             videoUnits: videoUnits
                                          });
      this.playerComponent =
        ReactDOM.render(playerView, document.getElementById('player'));

    });
  }
}
