defmodule Exopticon.Repo.Migrations.ChangeTimeToSeparateColumns do
  use Ecto.Migration

  def change do
    alter table(:files) do
      add :begin_time, :utc_datetime
      add :end_time, :utc_datetime
      remove :time
    end
  end
end
