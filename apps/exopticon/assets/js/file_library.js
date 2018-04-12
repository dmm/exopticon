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

import {Duration, ZonedDateTime} from 'js-joda';

/**
 * FileLibrary stores and returns information about a sequence of
 * video files
 * @class
 *
 */
class FileLibrary {
  /**
   * @param {List} files - list of video files to put in new library
   */
  constructor(files) {
    this.files = this.parseFiles(files);
  }

  /**
   * @param {List} files - list of files to parse
   * @return {List} list of parsed files
   * @private
   */
  parseFiles(files) {
    return files.map((f) => {
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
      ZonedDateTime.parse(datetime);
    }

    let ret = null;

    this.files.forEach((f) => {
      if (f.begin_time.compareTo(time) <= 0) {
        if (f.end_time.compareTo(time) > 0) {
          ret = {
            file: f,
            offset: Duration.between(f.begin_time, time).toMillis(),
          };
          return;
        }
      }
    });
    return ret;
  }

  /**
   * @param {Number} fileId - Id of file to
   * @return {File} returns file that follows given fileId
   *
   */
  getNextFile(fileId) {
    let ret = null;
    for (let f of this.files) {
      if (f.id === fileId) {
        ret = f;
      }

      if (ret !== null) {
        ret = f;
        break;
      }
    }
    return ret;
  }
}

export default FileLibrary;
