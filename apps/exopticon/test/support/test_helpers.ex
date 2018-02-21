# This file is part of Exopticon (https://github.com/dmm/exopticon).
# Copyright (c) 2018 David Matthew Mattli
#
# Exopticon is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# Exopticon is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with Exopticon.  If not, see <http://www.gnu.org/licenses/>.

defmodule Exopticon.TestHelpers do
  alias Exopticon.Repo

  def insert_user(attrs \\ %{}) do
    changes = Dict.merge(%{
          name: "Some User",
          username: "user#{Base.encode16(:crypto.rand_bytes(8))}",
          password: "supersecret",
                         }, attrs)
    %Exopticon.Accounts.User{}
    |> Exopticon.Accounts.User.registration_changeset(changes)
    |> Repo.insert!()
  end
end
