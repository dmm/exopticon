/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2022 David Matthew Mattli <dmm@mattli.us>
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

use uuid::Uuid;

use super::Error;

/// Maximum number of members in a ``CameraGroup``. This has to be less
/// than ``i32::MAX``.
static MAX_MEMBER_COUNT: usize = 1024;

/// Error message when ``CameraGroup`` members exceed ``MAX_MEMBER_COUNT``
static MAX_MEMBERS_ERROR_MESSAGE: &str = "Maximum number of CameraGroup members exceeded";

#[derive(Eq, PartialEq, Debug)]
#[non_exhaustive]
pub struct CameraGroup {
    pub name: String,
    pub members: Vec<Uuid>,
}

impl CameraGroup {
    pub fn new(name: &str, members: Vec<Uuid>) -> Result<Self, Error> {
        if members.len() > MAX_MEMBER_COUNT {
            return Err(Error::Validation(String::from(MAX_MEMBERS_ERROR_MESSAGE)));
        }

        Ok(Self {
            name: String::from(name),
            members,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn new_sets_correct_name() {
        // Arrange
        let name = "TestGroup";

        // Act
        let res = CameraGroup::new(name, Vec::new());

        // Assert
        assert_eq!(name, res.unwrap().name);
    }

    #[test]
    pub fn new_adds_correct_members() {
        // Arrange
        let members = vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];

        // Act
        let res = CameraGroup::new("TestGroup", members.clone());

        // Assert
        assert_eq!(members, res.unwrap().members);
    }

    #[test]
    pub fn new_succeeds_when_exactly_right_members() {
        // Arrange
        let mut members = Vec::new();

        for _i in 0..MAX_MEMBER_COUNT {
            members.push(Uuid::new_v4());
        }

        // Act
        let res = CameraGroup::new("TestGroup", members);

        // Assert
        assert!(res.is_ok());
    }

    #[test]
    pub fn new_errors_when_too_many_members() {
        // Arrange
        let mut members = Vec::new();

        for _i in 0..=MAX_MEMBER_COUNT {
            members.push(Uuid::new_v4());
        }

        // Act
        let res = CameraGroup::new("TestGroup", members);

        // Assert
        assert_eq!(
            res,
            Err(Error::Validation(String::from(MAX_MEMBERS_ERROR_MESSAGE)))
        );
    }
}
