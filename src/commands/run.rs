use std::{
    collections::HashMap,
    fs::remove_file,
    process::{Command, Stdio},
    str::FromStr,
};
use yaml_rust::Yaml;

use crate::{
    command::CommandInterface,
    config::{
        validation_rules::{is_array::IsArray, is_string::IsString, one_of::OneOf},
        validator::{arguments_are_named, validate_args, validate_named_args},
    },
    task_runner::TaskRunnerMode,
    utils::{
        shell::{create_script_file, Shell},
        terminal::set_environment_variables,
    },
};

pub struct RunCommand {}

fn get_commands_from_yaml(args: Yaml) -> Vec<String> {
    return if args.is_array() {
        args.as_vec()
            .unwrap()
            .iter()
            .map(|command| command.as_str().unwrap().to_string())
            .collect()
    } else {
        vec![args.as_str().unwrap().to_string()]
    };
}

fn get_commands(args: Yaml, mode: TaskRunnerMode) -> Result<Vec<String>, String> {
    let is_str_or_array = OneOf {
        rules: vec![Box::new(IsArray {}), Box::new(IsString {})],
    };
    let rules = vec![&is_str_or_array];

    let method_name = mode.to_string();

    if arguments_are_named(Some(&args)) {
        let validation =
            validate_named_args(args.clone(), HashMap::from([(method_name.clone(), rules)]));

        if let Err(err_validation) = validation {
            return Err(err_validation);
        }

        return Ok(get_commands_from_yaml(
            args.as_hash()
                .unwrap()
                .get(&Yaml::String(method_name))
                .unwrap()
                .to_owned(),
        ));
    }

    let validation = validate_args(Some(&args), rules);
    if let Err(err_validation) = validation {
        return Err(err_validation);
    }

    Ok(get_commands_from_yaml(args))
}

fn run_commands(
    commands: &Yaml,
    shell: &str,
    mode: TaskRunnerMode,
    temp_dir: &str,
) -> Result<String, String> {
    let parsed_commands = get_commands(commands.clone(), mode);
    if let Err(err) = parsed_commands {
        return Err(err);
    }
    let parsed_commands = parsed_commands.unwrap();

    let temp_script = create_script_file(
        Shell::from_str(shell).unwrap_or(Shell::Bash),
        parsed_commands,
        temp_dir,
    );

    if let Err(err) = temp_script {
        return Err(err);
    }
    let temp_script = temp_script.unwrap();

    let command = Command::new(shell)
        .arg("-c")
        .arg(&temp_script)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output();

    remove_file(temp_script).ok();

    if let Err(err_command) = command {
        return Err(err_command.to_string());
    }

    let output = command.unwrap();

    let mut stdout = String::from_utf8(output.stdout).unwrap_or_else(|_| String::from("OK"));
    if stdout.is_empty() {
        stdout = String::from("\n");
    }

    if !output.status.success() {
        let error_msg = String::from_utf8(output.stderr).unwrap_or_else(|e| e.to_string());

        return Err(if error_msg.is_empty() {
            stdout
        } else {
            error_msg
        });
    }

    Ok(stdout)
}

fn run_task(
    mode: TaskRunnerMode,
    args: Yaml,
    temp_dir: &str,
    default_shell: &Shell,
) -> Result<(), String> {
    let parameters = args.as_hash().unwrap();

    let param_commands = parameters
        .get(&Yaml::String(String::from("commands")))
        .unwrap();

    let default_shell = Yaml::String(default_shell.to_string());
    let param_shell = parameters
        .get(&Yaml::String(String::from("shell")))
        .unwrap_or(&default_shell)
        .as_str()
        .unwrap();

    if let Err(err_env) = set_environment_variables(&args) {
        return Err(err_env);
    }

    let result = run_commands(param_commands, param_shell, mode, temp_dir);

    if let Err(err_result) = result {
        return Err(err_result);
    }

    let result = result.unwrap();

    result.split('\n').for_each(|line| println!("{}", line));

    Ok(())
}

impl CommandInterface for RunCommand {
    fn install(&self, args: Yaml, temp_dir: &str, default_shell: &Shell) -> Result<(), String> {
        run_task(TaskRunnerMode::Install, args, temp_dir, default_shell)
    }

    fn uninstall(&self, args: Yaml, temp_dir: &str, default_shell: &Shell) -> Result<(), String> {
        run_task(TaskRunnerMode::Uninstall, args, temp_dir, default_shell)
    }

    fn update(&self, args: Yaml, temp_dir: &str, default_shell: &Shell) -> Result<(), String> {
        run_task(TaskRunnerMode::Update, args, temp_dir, default_shell)
    }
}

#[cfg(test)]
mod test {
    use yaml_rust::yaml::Hash;

    use super::*;

    #[test]
    fn it_gets_command_from_string() {
        let command = "echo hello";

        let commands = get_commands(Yaml::String(command.to_string()), TaskRunnerMode::Install);

        assert!(commands.is_ok());
        assert_eq!(vec![command.to_string()], commands.unwrap());
    }

    #[test]
    fn it_gets_commands_from_array() {
        let commands = Yaml::Array(vec![
            Yaml::String(String::from("command1")),
            Yaml::String(String::from("command2")),
        ]);

        let commands = get_commands(commands.clone(), TaskRunnerMode::Install);
        assert!(commands.is_ok());
        assert_eq!(
            commands.unwrap(),
            vec![String::from("command1"), String::from("command2")]
        );
    }

    #[test]
    fn it_gets_install_commands() {
        let mut commands = Hash::new();
        commands.insert(
            Yaml::String("install".to_string()),
            Yaml::Array(vec![
                Yaml::String("command1".to_string()),
                Yaml::String("command2".to_string()),
            ]),
        );

        let commands = get_commands(Yaml::Hash(commands.clone()), TaskRunnerMode::Install);
        assert!(commands.is_ok());
        assert_eq!(
            commands.unwrap(),
            vec![String::from("command1"), String::from("command2")]
        );
    }

    #[test]
    fn it_gets_install_command_string() {
        let mut commands = Hash::new();
        commands.insert(
            Yaml::String("install".to_string()),
            Yaml::String(String::from("command1")),
        );

        let commands = get_commands(Yaml::Hash(commands.clone()), TaskRunnerMode::Install);
        assert!(commands.is_ok());
        assert_eq!(commands.unwrap(), vec![String::from("command1")]);
    }

    #[test]
    fn it_gets_uninstall_commands() {
        let mut commands = Hash::new();
        commands.insert(
            Yaml::String("uninstall".to_string()),
            Yaml::Array(vec![
                Yaml::String("command1".to_string()),
                Yaml::String("command2".to_string()),
            ]),
        );

        let commands = get_commands(Yaml::Hash(commands.clone()), TaskRunnerMode::Uninstall);
        assert!(commands.is_ok());
        assert_eq!(
            commands.unwrap(),
            vec![String::from("command1"), String::from("command2")]
        );
    }

    #[test]
    fn it_gets_uninstall_command_string() {
        let mut commands = Hash::new();
        commands.insert(
            Yaml::String("uninstall".to_string()),
            Yaml::String(String::from("command1")),
        );

        let commands = get_commands(Yaml::Hash(commands.clone()), TaskRunnerMode::Uninstall);
        assert!(commands.is_ok());
        assert_eq!(commands.unwrap(), vec![String::from("command1")]);
    }

    #[test]
    fn it_gets_update_commands() {
        let mut commands = Hash::new();
        commands.insert(
            Yaml::String("update".to_string()),
            Yaml::Array(vec![
                Yaml::String("command1".to_string()),
                Yaml::String("command2".to_string()),
            ]),
        );

        let commands = get_commands(Yaml::Hash(commands.clone()), TaskRunnerMode::Update);
        assert!(commands.is_ok());
        assert_eq!(
            commands.unwrap(),
            vec![String::from("command1"), String::from("command2")]
        );
    }

    #[test]
    fn it_gets_update_command_string() {
        let mut commands = Hash::new();
        commands.insert(
            Yaml::String("update".to_string()),
            Yaml::String(String::from("command1")),
        );

        let commands = get_commands(Yaml::Hash(commands.clone()), TaskRunnerMode::Update);
        assert!(commands.is_ok());
        assert_eq!(commands.unwrap(), vec![String::from("command1")]);
    }

    #[test]
    fn it_fails_when_method_is_not_defined() {
        let mut commands = Hash::new();
        commands.insert(
            Yaml::String("invalid".to_string()),
            Yaml::String(String::from("command1")),
        );

        let commands = get_commands(Yaml::Hash(commands.clone()), TaskRunnerMode::Install);
        assert!(commands.is_err());
        assert_eq!(
            commands.unwrap_err(),
            String::from("install: OneOf: argument must be an array | argument must be a string")
        );
    }
}
