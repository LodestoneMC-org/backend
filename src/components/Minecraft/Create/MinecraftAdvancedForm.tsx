import InputField from 'components/Atoms/Form/InputField';
import { Field } from 'formik';

export default function MinecraftAdvancedForm() {
  return (
    <>
      <h1 className="text-larger font-bold tracking-tight text-gray-300">
        Advanced Settings
      </h1>
      <p>
        Advanced settings for your minecraft server.
        <br />
      </p>
      <div className="mt-10 flex flex-col gap-16 text-left">
        <div className="flex flex-row justify-evenly gap-8">
          <InputField type="number" name="min_ram" label="Minimum Ram" />
          <InputField type="number" name="max_ram" label="Maximum Ram" />
        </div>
        <InputField type="text" name="cmd_args" label="Command arguments" />
        <InputField type="text" name="auto_start" label="Auto Start" />
        <InputField
          type="text"
          name="restart_on_crash"
          label="Restart On Crash"
        />
      </div>
    </>
  );
}