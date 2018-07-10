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
  Duration,
  ZonedDateTime,
} from 'js-joda';
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
    this.state = {
      availability: {
        availability: [],
        begin_time: ZonedDateTime.parse('9999-12-31T23:59:59.999Z'),
        end_time: ZonedDateTime.parse('1970-01-01T00:00:00.000Z'),
        current_time: ZonedDateTime.parse('1970-01-01T00:00:00.000Z'),
      },
      timeLabel: '',
    };
    this.gaps = [];

    this.onMouseMove = this.onMouseMove.bind(this);
    this.onMouseLeave = this.onMouseLeave.bind(this);
    this.onMouseUp = this.onMouseUp.bind(this);
  }

  /**
   * onMouseMove
   * @param {object} e
   */
  onMouseMove(e) {
    this._label.style.display = 'block';
    const ratio = (e.clientX - e.currentTarget.offsetLeft)
          / e.currentTarget.clientWidth;

    const timeOffset =
          Duration.between(this.state.availability.begin_time,
                           this.state.availability.end_time).toMillis()
          * ratio;

    const newTime = this.state.availability.begin_time
          .plusSeconds(timeOffset / 1000);
    const formattedDate =
          newTime.format(DateTimeFormatter.ofPattern('yyyy-MM-dd HH:mm:ss'));
    this.setState({
      timeLabel: formattedDate,
    });
  }

  /**
   * onMouseLeave
   */
  onMouseLeave() {
    this._label.style.display = 'none';
  }


  /**
   * onMouseUp
   * @param {Object} e - event object
   * @private
   */
  onMouseUp(e) {
    const ratio = (e.clientX - e.currentTarget.offsetLeft)
          / e.currentTarget.clientWidth;

    const timeOffset =
          Duration.between(this.state.availability.begin_time,
                           this.state.availability.end_time).toMillis()
          * ratio;
    const newTime = this.state.availability.begin_time
          .plusSeconds(timeOffset / 1000);
    console.log(newTime.toString());
  }

  /**
   * react render function
   * @return {object} react
   */
  render() {
    const chunks = this.state.availability.availability;
    const duration =
          Duration.between(this.state.availability.begin_time,
                           this.state.availability.end_time).toMillis();
    let elm = [];
    chunks.forEach((c, i) => {
      const chunkLength = Duration.between(c.begin_time, c.end_time).toMillis();
      const percentage = ((chunkLength / duration) * 100);
      elm.push((
        <div className={`progress-element ${c.type}`}
        key={i} style={{width: percentage+'%'}} />
      ));
    });

    return (
      <div className='progress-bar'
           onMouseMove={this.onMouseMove}
           onMouseLeave={this.onMouseLeave}
           onMouseUp={this.onMouseUp}
           >
        { elm }
        <div className='time-label'
             ref={
               (el) => {
                 this._label = el;
               }
          }>
          <span>{this.state.timeLabel}</span>
        </div>
      </div>
    );
  }
}

export default ProgressBar;
