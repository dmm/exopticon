/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2020 David Matthew Mattli <dmm@mattli.us>
 *
 * This file is part of Exopticon.
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

import { CameraId } from "./camera";

export const ALL_GROUP_ID = "34e79812-df14-4773-a9f4-f766c799aa62";

export type CameraGroupId = string;

export class CameraGroup {
  id?: CameraGroupId;
  name: string;
  members: CameraId[];

  constructor() {
    this.id = null;
    this.name = "";
    this.members = new Array();
  }
}
