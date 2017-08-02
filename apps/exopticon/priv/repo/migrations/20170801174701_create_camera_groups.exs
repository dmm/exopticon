defmodule Exopticon.Repo.Migrations.CreateCameraGroups do
  use Ecto.Migration

  def change do
    create table(:camera_groups) do
      add :name, :string
      add :storage_path, :string
      add :max_storage_size, :integer

      timestamps()
    end

  end
end
