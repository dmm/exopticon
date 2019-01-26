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
  Duration,
  ZonedDateTime,
  ZoneOffset,
} from 'js-joda';
import jsJodaTimeZone from 'js-joda-timezone';
import React from 'react';
import ReactDOM from 'react-dom';

import MainView from '../main';
import CameraPlayerView from '../../components/camera_player_view';
import VideoUnitRepository from '../../repositories/video_unit_repository';

jsJodaUse(jsJodaTimeZone);

/**
 * CameraShow class
 * Implements page that shows a single camera
 */
export default class view extends MainView {
  /**
   * CameraPlayback constructor
   */
  constructor() {
    super();
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

    let videoUnitRepository = new VideoUnitRepository(window.apiRoot);

    let playerView =
        React.createElement(CameraPlayerView,
                            {
                              cameraId: cameraId,
                              initialBeginTime: begin,
                              initialEndTime: end,
                              videoUnitRepository: videoUnitRepository,
                              timezone: window.userTimezone,
                            });
      this.playerComponent =
        ReactDOM.render(playerView, document.getElementById('player'));
  }
}
