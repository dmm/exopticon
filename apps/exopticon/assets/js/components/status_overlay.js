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

import PropTypes from 'prop-types';
import React from 'react';

import OverlayButton from './overlay_button';

import '../../css/components/status_overlay.css';

/**
 * StatusOverlay class - Implements an overlay showing camera status
 * when camera is not playing.
 */
class StatusOverlay extends React.Component {
  /**
   * StatusOverlay constructor
   * @param {object} props - accepts status and cameraName props
   */
  constructor(props) {
    super(props);
  }

  /**
   * react render function
   * @return {object} react object
   */
  render() {
    return (
        <div className="status-overlay">
        <OverlayButton
          label="👁"
          extra-class="camera-link"
          link-to=""
        />
          <div className="status-message">
            {this.props.status}: {this.props.cameraName}
          </div>
        </div>
    );
  }
}

StatusOverlay.propTypes = {
  status: PropTypes.string.isRequired,
  cameraName: PropTypes.string.isRequired,
};

export default StatusOverlay;
