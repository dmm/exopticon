# This file is a part of Exopticon, a free video surveillance tool. Visit
# https://exopticon.org for more information.
#
# Copyright (C) 2018 David Matthew Mattli
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

defmodule Exopticon.Video.CameraGroup do
  @moduledoc """
  Provides schema for Video.CameraGroup.
  """
  use Ecto.Schema
  import Ecto.Changeset
  alias Exopticon.Video.CameraGroup

  schema "camera_groups" do
    field(:max_storage_size, :integer)
    field(:name, :string)
    field(:storage_path, :string)

    timestamps()
  end

  @doc false
  def changeset(%CameraGroup{} = camera_group, attrs) do
    camera_group
    |> cast(attrs, [:name, :storage_path, :max_storage_size])
    |> validate_required([:name, :storage_path, :max_storage_size])
  end
end
