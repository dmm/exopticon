export interface Observation {
  id: number;
  videoUnitId: number;
  frameOffset: number;
  tag: string;
  details: string;
  score: number;
  ulX: number;
  ulY: number;
  lrX: number;
  lrY: number;
}
