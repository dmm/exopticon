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
  constructor(props) {
    super(props);
    this.state = {
      timeLabel: '',
      chunks: this.calculateAvailability(props.videoUnits,
                                         props.beginTime,
                                         props.endTime)
    };

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
          Duration.between(this.props.beginTime,
                           this.props.endTime).toMillis()
          * ratio;

    const newTime = this.props.beginTime
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
          Duration.between(this.props.beginTime,
                           this.props.endTime).toMillis()
          * ratio;
    const newTime = this.props.beginTime
          .plusSeconds(timeOffset / 1000);
    if (this.props.onClick) {
      this.props.onClick(newTime);
    }
  }

  /**
   * calculateAvailability
   * @param {Array} units - array of video units
   * @param {ZonedDateTime} begin_time
   * @param {ZonedDateTime} end_time
   */
  calculateAvailability(units, begin_time, end_time) {
    let availability = [];
    let last = undefined;

    units.forEach((u, i) => {
      let video_begin = ZonedDateTime.parse(u.begin_time);
      if (video_begin.isBefore(begin_time)) {
        video_begin = begin_time;
      }
      let video_end = ZonedDateTime.parse(u.end_time);
      if (video_end.isAfter(end_time)) {
        video_end = end_time;
      }

      availability.push({
        startOffsetMs: Duration.between(begin_time, video_begin).toMillis(),
        endOffsetMs: Duration.between(begin_time, video_end).toMillis(),
        type: 'video',
      });

      const next = units[i+1];
      if (next && Duration.between(video_end,
                                   ZonedDateTime.parse(next.begin_time)).toMillis() > 1000) {
        console.log('GAP');
        availability.push({
          startOffsetMs: Duration.between(begin_time, video_end
                                          .plus(Duration.ofMillis(1))).toMillis(),
          endOffsetMs: Duration.between(begin_time,
                                        ZonedDateTime
                                        .parse(next.begin_time)
                                        .minus(Duration.ofMillis(1))).toMillis(),
          type: 'no-video',
        });
      }
    });

    return availability;
  }

  /**
   * react render function
   * @return {object} react
   */
  render() {
    const duration = Duration.between(this.props.beginTime,
                                      this.props.endTime).toMillis();
    let elm = [];
    this.state.chunks.forEach((c, i) => {
      const chunkLength = c.endOffsetMs - c.startOffsetMs;
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
