defmodule Exopticon.Repo.Migrations.CreateAnnotations do
  use Ecto.Migration

  def change do
    create table(:annotations) do
      add :key, :string
      add :value, :string
      add :source, :string
      add :frame_index, :integer
      add :ul_x, :integer
      add :ul_y, :integer
      add :width, :integer
      add :height, :integer
      add :hd_filename, :string
      add :sd_filename, :string
      add :video_unit_id, references(:video_units, on_delete: :nothing)

      timestamps()
    end

    create index(:annotations, [:video_unit_id])
  end
end
