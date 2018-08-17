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

defmodule Exopticon.Video.Annotation do
  @moduledoc """
  Provides schema for Video.Annotation
  """
  use Ecto.Schema
  import Ecto.Changeset
  alias Exopticon.Video.Annotation

  schema "annotations" do
    field(:frame_index, :integer)
    field(:offset, :integer)
    field(:height, :integer)
    field(:key, :string)
    field(:source, :string)
    field(:ul_x, :integer)
    field(:ul_y, :integer)
    field(:value, :string)
    field(:width, :integer)
    field(:hd_filename, :string)
    field(:sd_filename, :string)

    belongs_to(:video_unit, Exopticon.Video.VideoUnit)

    timestamps()
  end

  @doc false
  def changeset(%Annotation{} = annotation, attrs) do
    annotation
    |> cast(attrs, [
      :key,
      :value,
      :source,
      :frame_index,
      :offset,
      :video_unit_id,
      :ul_x,
      :ul_y,
      :width,
      :height,
      :sd_filename,
      :hd_filename
    ])
    |> validate_required([
      :key,
      :value,
      :source,
      :frame_index,
      :offset,
      #      :video_unit_id,
      :ul_x,
      :ul_y,
      :width,
      :height
    ])
  end
end
