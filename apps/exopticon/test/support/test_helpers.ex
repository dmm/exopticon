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
  @moduledoc """
  Provides helpers for Exopticon unit tests
  """
  alias Exopticon.Accounts

  def insert_user(attrs \\ %{}) do
    changes =
      Map.merge(
        %{
          name: "Some User",
          username: "user#{Base.encode16(:crypto.strong_rand_bytes(8))}",
          password: "supersecret",
          timezone: "ETC/UTC"
        },
        attrs
      )

    %Exopticon.Accounts.User{}
    |> Exopticon.Accounts.User.registration_changeset(changes)
    |> Exopticon.Accounts.create_user()
  end

  def user_fixture(attrs \\ %{}) do
    {:ok, user} =
      attrs
      |> Enum.into(%{
        name: "Some User",
        username: "user#{System.unique_integer([:positive])}",
        password: "supersecret",
        timezone: "ETC/UTC"
      })
      |> Accounts.register_user()

    user
  end
end
