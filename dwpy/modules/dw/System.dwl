/**
* This module contains functions that allow you to interact with the underlying
* system.
*
*
* To use this module, you must import it to your DataWeave code, for example,
* by adding the line `import * from dw::System` to the header of your
* DataWeave script.
*/

%dw 2.0

/**
* Returns all the environment variables defined in the host system as an array of strings.
*
* === Example
*
* This example returns a Mac command console (`SHELL`) path. `SHELL` is one of
* the standard Mac environment variables. To return all the environment
* variables, you can use `dw::System::envVars()`.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import dw::System
* output application/json
* ---
* { "envVars" : dw::System::envVars().SHELL }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* { "envVars": "/bin/bash" }
* ----
*/
@RuntimePrivilege(requires = "Environment")
fun envVars(): Dictionary<String> = native("system::env")

/**
* Returns an environment variable with the specified name or `null` if the
* environment variable is not defined.
*
* === Parameters
*
* [%header, cols="1,3"]
* |===
* | Name | Description
* | variableName | String that provides the name of the environment variable.
* |===
*
* === Example
*
* This example returns a Mac command console (`SHELL`) path and returns `null`
* on `FAKE_ENV_VAR` (an undefined environment variable). `SHELL` is one of the
* standard Mac environment variables. Also notice that the `import` command
* enables you to call the function without prepending the module name to it.
*
* ==== Source
*
* [source,DataWeave,linenums]
* ----
* %dw 2.0
* import * from dw::System
* output application/json
* ---
* {
*     "envVars" : [
*        "real" : envVar("SHELL"),
*        "fake" : envVar("FAKE_ENV_VAR")
*     ]
* }
* ----
*
* ==== Output
*
* [source,JSON,linenums]
* ----
* "envVars": [
*   {
*     "real": "/bin/bash"
*   },
*   {
*     "fake": null
*   }
* ]
* ----
*/
fun envVar(variableName: String): String | Null = envVars()[variableName]
