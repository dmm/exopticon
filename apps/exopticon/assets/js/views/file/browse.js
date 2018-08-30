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

import React from 'react';
import ReactDOM from 'react-dom';
import {ZonedDateTime, ZoneOffset} from 'js-joda';

import FileBrowser from '../../components/file_browser.js';
import MainView from '../main';
import socket from '../../socket';


/**
 * FileBrowser class
 * Implements page that allows a file-centric view of video
 * recordings.
 */
export default class view extends MainView {
  /**
   * implements FileBrowser logic
   *
   */
  mount() {
    super.mount();

    this.browser = React.createElement(FileBrowser, {
      socket: socket,
    });
    this.browserComponent = ReactDOM.render(
      this.browser,
      document.getElementById('file-browser-mount'));

    let cameraId = parseInt(document.getElementById('singleCamera')
                            .getAttribute('data-id'), 10);

    const now = ZonedDateTime.now(ZoneOffset.UTC);
    const endTime = now.minusMinutes(now.minute()).plusHours(1);
    const beginTime = now.minusDays(1);

    fetch(`/v1/video_units/between?camera_id=${cameraId}&begin_time=${beginTime.toString()}&end_time=${endTime.toString()}`, {
      credentials: 'same-origin',
      headers: {
        'Content-Type': 'application/json',
      },
    }).then((response) => {
      return response.json();
    }).then((videoUnits) => {
      this.browserComponent.setState({videos: videoUnits});
    });
  }
}
