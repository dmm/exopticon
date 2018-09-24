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

/**
 * VideoUnitRepository fetches video units from the EXOPTICON server.
 * @class
 */
class VideoUnitRepository {
  /**
   * VideoUnit Repository
   * @param {string} urlRoot - root api path
   */
  constructor(urlRoot = '/') {
    this.urlRoot = urlRoot;
  }

  /**
   * @param {Number} cameraId - id of camera for which to fetch video units
   * @param {ZonedDateTime} beginTime - begin time, inclusive
   * @param {ZonedDateTime} endTime - end time, inclusive
   * @return {Promise} A promise representing the resulting video units.
   */
  fetchBetween(cameraId, beginTime, endTime) {
    let url = `${this.urlRoot}v1/video_units/between?camera_id=${cameraId}`
        + `&begin_time=${beginTime.toString()}&end_time=${endTime.toString()}`;
    return fetch(url, {
      credentials: 'same-origin',
      headers: {
        'Content-Type': 'application/json',
      },
    }).then((response) => {
      return response.json();
    });
  }
}

export default VideoUnitRepository;
