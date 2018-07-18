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

defmodule Exopticon.Video.VideoUnit do
  @moduledoc """
  Provides schema for Video.VideoUnit
  """
  use Ecto.Schema
  import Ecto.Changeset
  alias Exopticon.Video.VideoUnit


  schema "video_units" do
    field :begin_monotonic, :integer
    field :begin_time, :utc_datetime
    field :end_monotonic, :integer
    field :end_time, :utc_datetime
    field :monotonic_index, :integer
    field :camera_id, :id
    has_many :files, Exopticon.Video.File

    timestamps()
  end

  @doc false
  def changeset(%VideoUnit{} = video_unit, attrs) do
    video_unit
    |> cast(attrs, [:begin_time, :end_time, :begin_monotonic, :end_monotonic, :monotonic_index])
    |> validate_required([:begin_time, :end_time, :begin_monotonic, :end_monotonic, :monotonic_index])
  end
end
