import { ZonedDateTime } from '@js-joda/core'

import { Observation } from './observation';

export class VideoUnit {
  constructor(public obj: any) {
    this.id = obj.id
    this.cameraId = obj.id;
    this.beginTime = ZonedDateTime.parse(obj.beginTime + 'Z');
    this.endTime = ZonedDateTime.parse(obj.endTime + 'Z');
    this.observations = obj.observations;
  }

  id: number;
  cameraId: number;
  beginTime: ZonedDateTime;
  endTime: ZonedDateTime;
  observations: Observation[];
}
