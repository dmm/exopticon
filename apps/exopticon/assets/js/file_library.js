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
import {Duration, ZonedDateTime} from 'js-joda';

/**
 * VideoLibrary stores and returns information about a sequence of
 * video units
 * @class
 *
 */
class FileLibrary {
  /**
   * @param {List} files - list of video units to put in new library
   */
  constructor(files) {
    this.files = this.parseFiles(files);
  }

  /**
   * @param {List} videoUnits - list of video units to parse
   * @return {List} list of parsed video units
   * @private
   */
  parseFiles(videoUnits) {
    return videoUnits.map((f) => {
      {
        let f2 = Object.assign({}, f);
        f2.begin_time = ZonedDateTime.parse(f.begin_time);
        f2.end_time = ZonedDateTime.parse(f.end_time);
        return f2;
      }
    });
  }

  /**
   * @param {ZonedDateTime} datetime - time to get file for
   * @return {Object} returns object spanning specified time
   */
  getFileForTime(datetime) {
    let time = datetime;

    if (typeof datetime === 'string') {
      time = ZonedDateTime.parse(datetime);
    }

    let file = this.files.find((f) => {
      if (f.begin_time.compareTo(time) <= 0) {
        if (f.end_time.compareTo(time) > 0) {
          return true;
        };
      }
      return false;
    });

    if (file !== undefined) {
      return {
        file: file,
        offset: Duration.between(file.begin_time, time).toMillis(),
      };
    }

    return undefined;
  }

  /**
   * @param {Number} fileId - Id of file to
   * @return {File} returns file that follows given fileId
   *
   */
  getNextFile(fileId) {
    let ret = null;
    let i = 0;
    for (let f of this.files) {
      if (f.files[0].id === fileId) {
        ret = this.files[i+1];
      }

      if (ret !== null) {
        break;
      }
      i++;
    }
    return ret;
  }
}

export default FileLibrary;
