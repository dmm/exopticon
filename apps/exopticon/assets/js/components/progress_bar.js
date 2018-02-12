import { use as jsJodaUse, ZonedDateTime } from 'js-joda';
import jsJodaTimeZone from 'js-joda-timezone';

import React from 'react';
import ReactDOM from 'react-dom';

import './../../css/components/progress_bar.css';

jsJodaUse(jsJodaTimeZone);

class ProgressBar extends React.Component {
  constructor() {
    super();
    this.state = { files: [] };
    this.beginTime = ZonedDateTime.parse('9999-12-31T23:59:59.999Z');
    this.endTime = ZonedDateTime.parse('1970-01-01T00:00:00.000Z');
    this.gaps = [];
  }

  processFiles(files) {
    console.log('Processing ' + files.length + ' files.');
    files.forEach((f) => {
      const begin = ZonedDateTime.parse(f.begin_time);
      const end = end ? ZonedDateTime.parse(f.end_time) : null;
      if (begin.isBefore(this.beginTime)) {
        this.beginTime = begin;
      }

      if(end && end.isAfter(this.endTime)) {
        this.endTime = end;
      }
    });

    console.log(`Begin time: ${this.beginTime.toString()} End time: ${this.endTime.toString()}`);
  }

  render() {
    this.processFiles(this.state.files);
    return (
       <div className='progress-bar'></div>
    );
  }

}

export default ProgressBar;
