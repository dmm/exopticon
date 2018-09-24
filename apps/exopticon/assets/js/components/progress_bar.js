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
  Duration,
} from 'js-joda';
import jsJodaTimeZone from 'js-joda-timezone';
import memoizeOne from 'memoize-one';
import PropTypes from 'prop-types';
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
   * @param {object} props
   */
  constructor(props) {
    super(props);

    this.state = {
      timeLabel: '',
      hoverPosition: {x: 0, y: 0},
    };

    this.calculateAvailability = memoizeOne(this.calculateAvailability);

    this.onMouseMove = this.onMouseMove.bind(this);
    this.onMouseLeave = this.onMouseLeave.bind(this);
    this.onMouseUp = this.onMouseUp.bind(this);
  }

  /**
   * onMouseMove
   * @param {object} e
   */
  onMouseMove(e) {
    const ratio = (e.clientX - e.currentTarget.offsetLeft)
          / e.currentTarget.clientWidth;

    const timeOffset =
          Duration.between(this.props.beginTime,
                           this.props.endTime).toMillis()
          * ratio;

    const newTime = this.props.beginTime
          .plusSeconds(timeOffset / 1000);
    this.props.onTimeHover(newTime);
    console.log(newTime.toString());
    this.setState({
      hoverTime: newTime,
      hoverPosition: {x: e.clientX, y: e.clientY - 35},
    });
  }

  /**
   * onMouseLeave
   */
  onMouseLeave() {
    this.props.onTimeLeave();
    this.setState({
      hoverTime: undefined,
    });
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
   * @param {ZonedDateTime} beginTime
   * @param {ZonedDateTime} endTime
   * @return {object} availability
   */
  calculateAvailability(units, beginTime, endTime) {
    let availability = [];

    units.forEach((u, i) => {
      let videoBegin = u.begin_time;
      if (videoBegin.isBefore(videoBegin)) {
        videoBegin = beginTime;
      }
      let videoEnd = u.end_time;
      if (videoEnd.isAfter(endTime)) {
        videoEnd = endTime;
      }

      availability.push({
        startOffsetMs: Duration.between(beginTime, videoBegin).toMillis(),
        endOffsetMs: Duration.between(beginTime, videoEnd).toMillis(),
        type: 'video',
      });

      const next = units[i+1];
      if (next && Duration.between(videoEnd,
                                   next.begin_time).toMillis() > 1000) {
        availability.push({
          startOffsetMs: Duration.between(beginTime, videoEnd
                                          .plus(Duration.ofMillis(1)))
            .toMillis(),
          endOffsetMs: Duration.between(beginTime,
                                        next.begin_time
                                        .minus(Duration.ofMillis(1)))
            .toMillis(),
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
    const chunks = this.calculateAvailability(this.props.videoUnits,
                                              this.props.beginTime,
                                              this.props.endTime);

    let elm = [];
    chunks.forEach((c, i) => {
      const chunkLength = c.endOffsetMs - c.startOffsetMs;
      const percentage = ((chunkLength / duration) * 100);
      elm.push((
        <div className={`progress-element ${c.type}`}
             key={i} style={{width: percentage+'%'}} />
      ));
    });

    let progressElm = (<div className='progress-marker'
                       style={{left: this.props.progress+'%'}}
                       ></div>);

    let hoverTime = this.state.hoverTime ? (
      <div className="hover-time"
           style={{
             top: this.state.hoverPosition.y,
             left: this.state.hoverPosition.x,
           }}>
        {this.props.formatLocal(this.state.hoverTime)}
      </div>
    ) : null;

    return (
      <div className='progress-wrapper'>
        <div className='progress-bar'
             onMouseMove={this.onMouseMove}
             onMouseLeave={this.onMouseLeave}
             onMouseUp={this.onMouseUp}
             onTouchEnd={this.onMouseUp}
             >
          { progressElm }
          { elm }
        </div>
        { hoverTime }
      </div>
    );
  }
}

ProgressBar.propTypes = {
  beginTime: PropTypes.object.isRequired,
  endTime: PropTypes.object.isRequired,
  formatLocal: PropTypes.func.isRequired,
  onClick: PropTypes.func.isRequired,
  onTimeHover: PropTypes.func,
  onTimeLeave: PropTypes.func,
  progress: PropTypes.number,
  videoUnits: PropTypes.array.isRequired,
};

ProgressBar.defaultProps = {
  extraClasses: '',
  onTimeHover: ()=>{},
  onTimeLeave: ()=>{},
  onClick: ()=>{},
  progress: -1,
};

export default ProgressBar;
