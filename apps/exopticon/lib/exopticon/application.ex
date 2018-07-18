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

defmodule Exopticon.Application do
  @moduledoc """
  Provides main application initialization for Exopticon.
  """
  use Application

  import Ecto.Query

  # See https://hexdocs.pm/elixir/Application.html
  # for more information on OTP Applications
  def start(_type, _args) do
    import Supervisor.Spec

    # Define workers and child supervisors to be supervised
    children = [
      # Start the Ecto repository
      supervisor(Exopticon.Repo, []),
      # Start the endpoint when the application starts
      supervisor(ExopticonWeb.Endpoint, []),
      supervisor(Registry, [:unique, Registry.PlayerRegistry], id: :PlayerRegistry),
      supervisor(Registry, [:unique, Registry.CameraRegistry], id: :CameraRegistry),
      supervisor(Exopticon.CameraSupervisor, []),
      supervisor(Exopticon.PlaybackSupervisor, []),
      supervisor(Exopticon.Video.FileDeletionSupervisor, [])
    ]

    # See https://hexdocs.pm/elixir/Supervisor.html
    # for other strategies and supported options
    opts = [strategy: :one_for_one, name: Exopticon.Supervisor]
    Supervisor.start_link(children, opts)
  end

  def start_phase(:start, :normal, [5]) do
    IO.puts("start called!")

    cameras =
      Exopticon.Repo.all(
        from(
          camera in Exopticon.Video.Camera,
          where: camera.mode == "enabled",
          preload: [:camera_group]
        )
      )

    # Start Cameras
    Exopticon.CameraSupervisor.start_all_cameras(cameras)

    :ok
  end

  def start_phase(phase, start_type, phase_args),
    do:
      IO.puts(
        "top_app:start_phase(#{inspect(phase)},#{inspect(start_type)},#{inspect(phase_args)})."
      )

  # Tell Phoenix to update the endpoint configuration
  # whenever the application is updated.
  def config_change(changed, _new, removed) do
    ExopticonWeb.Endpoint.config_change(changed, removed)
    :ok
  end
end
