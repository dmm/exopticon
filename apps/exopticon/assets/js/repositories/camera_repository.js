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
 * CameraRepository fetches cameras and information about camera
 * recordings
 * @class
 */
class CameraRepository {
  /**
   * constructor for CameraRepository
   * @param {string} urlRoot - root path of api
   */
  constructor(urlRoot = '/') {
    this.urlRoot = urlRoot;
  }

  /**
   * fetches coverage information and available files
   * @param {number} cameraId - id of camera to get file for
   * @param {ZonedDateTime} beginTime - start of time to get availability for
   * @param {ZonedDateTime} endTime - end of time to get availability for
   * @return {Promise} Promise object represents the availability for the given
   *                   camera and interval.
   */
  getAvailability(cameraId, beginTime, endTime) {
    let url = `${this.urlRoot}v1/cameras/${cameraId}/availability`;
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

export default CameraRepository;
