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

import '../../css/components/overlay_button.css';

/**
 * OverlayButton component
 * Implements a button for the camera overlay
 */
class OverlayButton extends React.Component {
  /**
   * react render function
   * @return {Object} OverlayButton
   */
  render() {
    return (
      <button className={'overlay-button ' + this.props.extraClass}
              onClick={this.props.onClick}>
        {this.props.label}
      </button>
    );
  }
}

OverlayButton.propTypes = {
  onClick: PropTypes.func,
  extraClass: PropTypes.string,
  label: PropTypes.string,
};

OverlayButton.defaultProps = {
  onClick: () => {},
  extraClass: '',
  label: '',
};

export default OverlayButton;
