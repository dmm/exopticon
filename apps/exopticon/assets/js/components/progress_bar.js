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

import {use as jsJodaUse, ZonedDateTime} from 'js-joda';
import jsJodaTimeZone from 'js-joda-timezone';
import React from 'react';

import './../../css/components/progress_bar.css';

jsJodaUse(jsJodaTimeZone);

/**
 * ProgressBar - class implementing a bar showing a time interval
 *
 */
class ProgressBar extends React.Component {
  /**
   * ProgressBar constructor
   */
  constructor() {
    super();
    this.state = {files: []};
    this.beginTime = ZonedDateTime.parse('9999-12-31T23:59:59.999Z');
    this.endTime = ZonedDateTime.parse('1970-01-01T00:00:00.000Z');
    this.gaps = [];
  }

  /**
   * processFiles
   * @param {Array} files
   *
   */
  processFiles(files) {
    console.log('Processing ' + files.length + ' files.');
    files.forEach((f) => {
      const begin = ZonedDateTime.parse(f.begin_time);
      const end = ZonedDateTime.parse(f.end_time);
      if (begin.isBefore(this.beginTime)) {
        this.beginTime = begin;
      }

      if (end.isAfter(this.endTime)) {
        this.endTime = end;
      }
    });

    console.log(`Begin time: ${this.beginTime.toString()}`
                + `End time: ${this.endTime.toString()}`);
  }
  /**
   * react render function
   * @return {object} react
   */
  render() {
    this.processFiles(this.state.files);
    return (
       <div className='progress-bar'></div>
    );
  }
}

export default ProgressBar;
