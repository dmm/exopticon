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

defmodule Exopticon.PlaybackSupervisor do
  @moduledoc """
  Provides supervisor for playback port.
  """
  use Supervisor

  def start_link do
    Supervisor.start_link(__MODULE__, [], name: __MODULE__)
  end

  def init(_) do
    children = [
      worker(Exopticon.PlaybackPort, [], restart: :transient)
    ]

    supervise(children, strategy: :simple_one_for_one)
  end

  def start_playback({id, file, offset}) do
    IO.puts("Starting playback port..." <> id)
    args = {id, file.filename, offset}
    Supervisor.start_child(Exopticon.PlaybackSupervisor, [args])
  end

  def stop_playback(id) do
    regs = Registry.lookup(Registry.PlayerRegistry, id)
    pids = Enum.map(regs, fn {pid, _} -> pid end)

    Enum.map(pids, fn p ->
      Supervisor.terminate_child(Exopticon.PlaybackSupervisor, p)
    end)
  end
end
