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

defmodule Exopticon.CameraSupervisor do
  @moduledoc """
  Provides supervisor for capture port processes.
  """
  use Supervisor

  def start_link do
    Supervisor.start_link(__MODULE__, [], name: __MODULE__)
  end

  def init(_) do
    children = [
      worker(Exopticon.CapturePort, [], restart: :permanent)
    ]

    supervise(children, strategy: :simple_one_for_one)
  end

  def start_all_cameras([]) do
  end

  def start_all_cameras(cameras) do
    [cam | tail] = cameras

    args = {
      cam.id,
      cam.rtsp_url,
      cam.fps,
      cam.camera_group.storage_path
    }

    regs = Registry.lookup(Registry.CameraRegistry, cam.id)

    if regs == [] do
      Supervisor.start_child(Exopticon.CameraSupervisor, [args])
    end

    start_all_cameras(tail)
  end

  def stop_camera(id) do
    regs = Registry.lookup(Registry.CameraRegistry, id)
    pids = Enum.map(regs, fn {pid, _} -> pid end)

    Enum.map(pids, fn p ->
      Supervisor.terminate_child(Exopticon.CameraSupervisor, p)
    end)
  end
end
