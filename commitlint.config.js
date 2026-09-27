// Conventional Commits with the house style from CONTRIBUTING.md. Runs from lefthook (commit-msg).
export default {
  extends: ["@commitlint/config-conventional"],
  rules: {
    "header-max-length": [2, "always", 72],
    "scope-case": [2, "always", "kebab-case"],
    "body-max-line-length": [1, "always", 100],
    "footer-max-line-length": [0],
  },
};
