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

defmodule Exopticon.Video.Camera do
  @moduledoc """
  Provides schema for Video.Camera
  """
  use Ecto.Schema
  import Ecto.Changeset
  alias Exopticon.Video.Camera

  schema "cameras" do
    field(:fps, :integer)
    field(:ip, :string)
    field(:mac, :string)
    field(:name, :string)
    field(:onvif_port, :integer)
    field(:password, :string)
    field(:rtsp_url, :string)
    field(:type, :string)
    field(:ptz_type, :string)
    field(:ptz_profile_token, :string)
    field(:username, :string)
    field(:mode, :string)
    belongs_to(:camera_group, Exopticon.Video.CameraGroup)

    timestamps()
  end

  @doc false
  def changeset(%Camera{} = camera, attrs) do
    camera
    |> cast(attrs, [
      :name,
      :ip,
      :onvif_port,
      :fps,
      :mac,
      :username,
      :password,
      :rtsp_url,
      :type,
      :ptz_type,
      :ptz_profile_token,
      :mode
    ])
    |> validate_required([:name, :ip, :fps, :mac, :username, :type, :mode])
  end
end
